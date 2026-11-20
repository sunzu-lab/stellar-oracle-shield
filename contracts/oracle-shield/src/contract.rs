mod events;
mod parameters_v1;
mod time_locked;

use {
    crate::score::{AggregatedScore, Score, TimestampedScore},
    events::*,
    soroban_sdk::{
        Address, BytesN, ContractExecutable, Env, Map, Vec, contract, contractimpl, contractmeta,
        contracttype,
    },
    stellar_oracle_shield_client::{
        Error, GovernanceParameters, GovernanceParametersConstraints, Parameters, Status,
    },
    time_locked::*,
};

contractmeta!(key = "name", val = env!("CARGO_PKG_NAME"));
contractmeta!(key = "version", val = env!("CARGO_PKG_VERSION"));
contractmeta!(key = "description", val = env!("CARGO_PKG_DESCRIPTION"));
contractmeta!(key = "license", val = env!("CARGO_PKG_LICENSE"));

/// contract data storage key
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
enum DataKey {
    /// administrator
    Admin,
    /// authorized operators
    /// legacy -- now part of Parameters since version 2 --
    Operators,
    /// contract parameter set version 1
    /// legacy, deprecated by ParametersV2
    Parameters,
    /// contract parameter set version 2
    ParametersV2,
    /// contract governance parameters
    GovernanceParameters,
    /// contract governance parameters constraints (immutable)
    GovernanceParametersConstraints,
}

/// scheduled operations storage keys
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
enum ScheduledKey {
    /// pending administrator
    Admin,
    /// pending parameters
    Parameters,
    /// pending governance parameters
    GovernanceParameters,
    /// pending upgrade
    Upgrade,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Pair(Address, Address);

/// latest report by an operator for an ordered pair
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct OperatorScoreKey(Pair, Address);

const VERSION: (u32, u32, u32) = (
    parse_version(env!("CARGO_PKG_VERSION_MAJOR")),
    parse_version(env!("CARGO_PKG_VERSION_MINOR")),
    parse_version(env!("CARGO_PKG_VERSION_PATCH")),
);

/// oracle shield main contract
#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    // SECURITY AUDIT NOTE
    //
    // Soroban native constructor (CAP-0058, Protocol 22+).
    // `__constructor` is a reserved lifecycle function invoked exclusively
    // by the Soroban host during contract creation.
    //
    // It CANNOT be invoked directly after deployment, even though it is
    // declared `pub` inside `#[contractimpl]`.
    //
    // Reinitialization attacks through `__constructor` are prevented
    // by the Soroban runtime. No explicit initialization guard is needed.
    //
    // Reference: https://github.com/stellar/stellar-protocol/blob/master/core/cap-0058.md
    /// initialize contract
    /// - set administator address, `admin`
    /// - optionally set initial parameters -- `max_staleness`, `max_deviation`, `quorum`, `operators` --
    /// - optionally set initial governance parameters -- `timelock_delay_seconds` --
    /// - optionally set immutable governance parameters constraints -- `min_timelock_delay_seconds`, `max_timelock_delay_seconds`
    pub fn __constructor(
        env: Env,
        admin: Address,
        parameters: Option<Parameters>,
        governance_parameters: Option<GovernanceParameters>,
        governance_constraints: Option<GovernanceParametersConstraints>,
    ) -> Result<(), Error> {
        admin.require_auth();
        Self::set_admin(&env, admin);

        let default_parameters = Parameters::default_with_env(&env);
        let parameters = parameters.unwrap_or(default_parameters);
        parameters.is_valid()?;
        Self::set_parameters(&env, parameters);

        let governance_parameters_constraints = governance_constraints.unwrap_or_default();
        governance_parameters_constraints.is_valid(&env)?;
        Self::set_governance_parameters_constraints(
            &env,
            governance_parameters_constraints.clone(),
        );

        let governance_parameters = governance_parameters.unwrap_or_default();
        governance_parameters.is_valid(&env, &governance_parameters_constraints)?;
        Self::set_governance_parameters(&env, governance_parameters);
        Ok(())
    }
}

impl Contract {
    fn version() -> (u32, u32, u32) {
        VERSION
    }

    fn set_parameters(env: &Env, parameters: Parameters) {
        env.storage()
            .instance()
            .set(&DataKey::ParametersV2, &parameters);

        ParametersChange { parameters }.publish(env);
    }

    fn get_parameters(env: &Env) -> Parameters {
        env.storage()
            .instance()
            .get(&DataKey::ParametersV2)
            .unwrap_or_else(|| Self::upgrade_parameters(env))
    }

    fn upgrade_parameters(env: &Env) -> Parameters {
        // post upgrade migration
        // set parameters v2 from parameters and operators
        let storage = env.storage().instance();

        let v1: parameters_v1::Parameters = storage.get(&DataKey::Parameters).unwrap_or_default();

        let default_operators = || Map::new(env);
        let operators: Map<Address, ()> = storage
            .get(&DataKey::Operators)
            .unwrap_or_else(default_operators);

        let parameters = Parameters {
            max_staleness: v1.max_staleness,
            max_deviation: v1.max_deviation,
            quorum: v1.quorum,
            operators,
        };

        Self::set_parameters(env, parameters.clone());
        // cleanup
        storage.remove(&DataKey::Parameters);
        storage.remove(&DataKey::Operators);

        parameters
    }

    fn schedule<T>(env: &Env, key: &ScheduledKey, value: T) -> Result<T::TimeLocked, Error>
    where
        T: IntoTimeLocked,
    {
        let storage = env.storage().temporary();
        if storage.has(key) {
            return Err(Error::ScheduledOperationInProgress);
        }

        let governance_parameters = Self::get_governance_params(env);
        let now = env.ledger().timestamp();
        let valid_after = now + governance_parameters.timelock_delay_seconds;
        let value = value.into_time_locked(valid_after);

        storage.set(key, &value);
        let ttl_extension = governance_parameters.required_ttl();
        storage.extend_ttl(key, ttl_extension, ttl_extension);
        Ok(value)
    }

    fn unschedule_if_valid<T>(env: &Env, key: &ScheduledKey) -> Result<T, Error>
    where
        T: IntoTimeLocked,
    {
        let storage = env.storage().temporary();
        let time_locked = storage
            .get::<ScheduledKey, T::TimeLocked>(key)
            .ok_or(Error::Expired)?;

        if env.ledger().timestamp() < time_locked.valid_after() {
            return Err(Error::TimeLocked);
        }

        storage.remove(key);
        Ok(time_locked.inner())
    }

    fn unschedule<T>(env: &Env, key: &ScheduledKey) -> Result<T, Error>
    where
        T: IntoTimeLocked,
    {
        let storage = env.storage().temporary();
        let time_locked: T::TimeLocked = storage.get(&key).ok_or(Error::Expired)?;

        storage.remove(key);
        Ok(time_locked.inner())
    }

    fn schedule_set_parameters(env: Env, parameters: Parameters) -> Result<(), Error> {
        parameters.is_valid()?;
        let parameters = Self::schedule(&env, &ScheduledKey::Parameters, parameters)?;

        let event = ParametersChangeInitiated { parameters };
        event.publish(&env);

        Ok(())
    }

    fn execute_set_parameters(env: Env) -> Result<(), Error> {
        let parameters: Parameters = Self::unschedule_if_valid(&env, &ScheduledKey::Parameters)?;
        Self::set_parameters(&env, parameters);

        Ok(())
    }

    fn cancel_set_parameters(env: Env) -> Result<(), Error> {
        let parameters: Parameters = Self::unschedule(&env, &ScheduledKey::Parameters)?;
        let event = ParametersChangeCancelled { parameters };
        event.publish(&env);

        Ok(())
    }

    fn get_scheduled_parameters(env: &Env) -> Option<TimeLockedParameters> {
        let storage = env.storage().temporary();
        storage.get(&ScheduledKey::Parameters)
    }

    fn schedule_set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error> {
        let mut parameters = Self::get_parameters(&env);
        parameters.max_staleness = max_staleness;
        Self::schedule_set_parameters(env, parameters)
    }

    fn schedule_set_max_deviation(env: Env, max_deviation: u32) -> Result<(), Error> {
        let mut parameters = Self::get_parameters(&env);
        parameters.max_deviation = max_deviation;
        Self::schedule_set_parameters(env, parameters)
    }

    fn schedule_set_quorum(env: Env, quorum: u32) -> Result<(), Error> {
        let mut parameters = Self::get_parameters(&env);
        parameters.quorum = quorum;
        Self::schedule_set_parameters(env, parameters)
    }

    fn schedule_add_operator(env: Env, operator: Address) -> Result<(), Error> {
        let mut parameters = Self::get_parameters(&env);
        parameters.operators.set(operator, ());
        Self::schedule_set_parameters(env, parameters)
    }

    fn schedule_remove_operator(env: Env, operator: Address) -> Result<(), Error> {
        let mut parameters = Self::get_parameters(&env);
        parameters.operators.remove(operator);
        Self::schedule_set_parameters(env, parameters)
    }

    fn schedule_set_operators(env: Env, operators: Map<Address, ()>) -> Result<(), Error> {
        let mut parameters = Self::get_parameters(&env);
        parameters.operators = operators;
        Self::schedule_set_parameters(env, parameters)
    }

    fn set_admin(env: &Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);

        let event = AdminTransfered { admin };
        event.publish(env);
    }

    fn get_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::MissingAdmin)
    }

    fn get_operators(env: &Env) -> Map<Address, ()> {
        Self::get_parameters(env).operators
    }

    fn require_operator_auth(env: &Env, operator: &Address) -> Result<(), Error> {
        let parameters = Self::get_parameters(env);
        if !parameters.operators.contains_key(operator.clone()) {
            return Err(Error::UnauthorizedOperator);
        }
        operator.require_auth();

        Ok(())
    }

    fn set_governance_parameters(env: &Env, parameters: GovernanceParameters) {
        env.storage()
            .instance()
            .set(&DataKey::GovernanceParameters, &parameters);

        GovernanceParamsChange { parameters }.publish(env);
    }

    fn get_governance_params(env: &Env) -> GovernanceParameters {
        env.storage()
            .instance()
            .get(&DataKey::GovernanceParameters)
            .unwrap_or_else(|| Self::upgrade_governance_parameters(env))
    }

    fn upgrade_governance_parameters(env: &Env) -> GovernanceParameters {
        // post upgrade migration
        // set governance parameters to defaults
        let governance_parameters = GovernanceParameters::default();
        Self::set_governance_parameters(env, governance_parameters);
        governance_parameters
    }

    fn set_governance_parameters_constraints(
        env: &Env,
        constraints: GovernanceParametersConstraints,
    ) {
        env.storage()
            .instance()
            .set(&DataKey::GovernanceParametersConstraints, &constraints);
    }

    fn get_governance_parameters_constraints(env: &Env) -> GovernanceParametersConstraints {
        env.storage()
            .instance()
            .get(&DataKey::GovernanceParametersConstraints)
            .unwrap_or_else(|| Self::upgrade_governance_parameters_constraints(env))
    }

    fn upgrade_governance_parameters_constraints(env: &Env) -> GovernanceParametersConstraints {
        // post upgrade migration
        // set governance parameters constraints to defaults
        let governance_parameters_constraints = GovernanceParametersConstraints::default();
        Self::set_governance_parameters_constraints(env, governance_parameters_constraints.clone());
        governance_parameters_constraints
    }

    fn schedule_set_governance_params(
        env: Env,
        parameters: GovernanceParameters,
    ) -> Result<(), Error> {
        let governance_parameters_constraints = Self::get_governance_parameters_constraints(&env);
        parameters.is_valid(&env, &governance_parameters_constraints)?;
        let parameters = Self::schedule(&env, &ScheduledKey::GovernanceParameters, parameters)?;

        let event = GovernanceParamsChangeInit { parameters };
        event.publish(&env);

        Ok(())
    }

    fn execute_set_governance_params(env: Env) -> Result<(), Error> {
        let parameters: GovernanceParameters =
            Self::unschedule_if_valid(&env, &ScheduledKey::GovernanceParameters)?;
        Self::set_governance_parameters(&env, parameters);

        Ok(())
    }

    fn cancel_set_governance_params(env: Env) -> Result<(), Error> {
        let parameters: GovernanceParameters =
            Self::unschedule(&env, &ScheduledKey::GovernanceParameters)?;

        let event = GovernanceParamsChangeCancel { parameters };
        event.publish(&env);

        Ok(())
    }

    fn get_scheduled_governance_params(env: &Env) -> Option<TimeLockedGovernanceParameters> {
        let storage = env.storage().temporary();
        storage.get(&ScheduledKey::GovernanceParameters)
    }

    fn set_scores(
        env: Env,
        operator: Address,
        scores: Vec<(Address, Address, u32)>,
    ) -> Result<(), Error> {
        for (base, quote, score) in scores {
            Self::set_score(&env, operator.clone(), base, quote, score)?;
        }
        Ok(())
    }

    fn set_score(
        env: &Env,
        operator: Address,
        base: Address,
        quote: Address,
        score: u32,
    ) -> Result<(), Error> {
        let pair = Pair(base, quote);
        let old_score = Self::get_inner_score(env, &pair);
        let ope_score = TimestampedScore::new(score, env.ledger().timestamp())?;
        let key = OperatorScoreKey(pair.clone(), operator);
        env.storage().temporary().set(&key, &ope_score);
        let new_score = Self::get_inner_score(env, &pair);

        if let Ok(score) = new_score {
            let (score_changed, status_changed) = if let Ok(os) = old_score {
                (
                    os.score.score != score.score.score,
                    os.status() != score.status(),
                )
            } else {
                (true, true)
            };

            if score_changed {
                ScoreChange {
                    base: pair.0.clone(),
                    quote: pair.1.clone(),
                    score: score.score.score,
                }
                .publish(env);
            }

            if status_changed {
                StatusChange {
                    base: pair.0.clone(),
                    quote: pair.1.clone(),
                    status: score.status(),
                }
                .publish(env);
            }
        }
        Ok(())
    }

    fn get_inner_score(env: &Env, pair: &Pair) -> Result<AggregatedScore, Error> {
        let parameters = Self::get_parameters(env);
        let now = env.ledger().timestamp();
        let mut values: Vec<u32> = Vec::new(env);
        let mut oldest_ts = now;
        let mut found_report = false;
        let operators = Self::get_operators(env);
        let num_operators = operators.len();

        for (operator, _) in operators.iter() {
            let key = OperatorScoreKey(pair.clone(), operator);
            if let Some(ts_score) = env.storage().temporary().get::<_, TimestampedScore>(&key) {
                found_report = true;
                let Some(age) = now.checked_sub(ts_score.ts) else {
                    continue;
                };
                if age > parameters.max_staleness {
                    continue;
                }
                oldest_ts = oldest_ts.min(ts_score.ts);
                let score = ts_score.score.score;
                let index = values.binary_search(score).unwrap_or_else(|i| i);
                values.insert(index, score);
            }
        }

        if values.is_empty() {
            return Err(if found_report {
                Error::StaleInput
            } else {
                Error::PairNotCovered
            });
        }

        let count = values.len();
        let middle = count / 2;
        let median = if count.is_multiple_of(2) {
            (values.get_unchecked(middle - 1) + values.get_unchecked(middle)) / 2
        } else {
            values.get_unchecked(middle)
        };
        let mut consensus = 0_u32;
        for value in values.iter() {
            if value.abs_diff(median) <= parameters.max_deviation {
                consensus += 1;
            }
        }

        if consensus < parameters.quorum {
            return Err(Error::QuorumNotReached);
        }
        let aggregated_score = AggregatedScore {
            score: Score::new(median)?,
            oldest_ts,
            ts: now,
            consensus,
            num_operators,
        };
        Ok(aggregated_score)
    }

    fn get_score(env: Env, base: Address, quote: Address) -> Result<u32, Error> {
        let score = Self::get_inner_score(&env, &Pair(base, quote))?;
        Ok(score.score.score)
    }

    fn get_status(env: Env, base: Address, quote: Address) -> Result<Status, Error> {
        let score = Self::get_inner_score(&env, &Pair(base, quote))?;
        Ok(score.score.into())
    }

    fn schedule_upgrade(env: Env, wasm_hash: BytesN<32>) -> Result<(), Error> {
        let wasm_hash = Self::schedule(&env, &ScheduledKey::Upgrade, wasm_hash)?;

        let event = UpgradeInitiated { wasm_hash };
        event.publish(&env);

        Ok(())
    }

    fn execute_upgrade(env: Env) -> Result<(), Error> {
        let wasm_hash: BytesN<32> = Self::unschedule_if_valid(&env, &ScheduledKey::Upgrade)?;

        // also remove scheduled operations signed in previous version
        let _ = Self::cancel_set_parameters(env.clone());
        let _ = Self::cancel_set_governance_params(env.clone());
        let _ = Self::cancel_admin_transfer(env.clone());

        let executable = ContractExecutable::Wasm(wasm_hash.clone());
        env.deployer().update_current_contract(executable);

        let event = UpgradeExecuted { wasm_hash };
        event.publish(&env);

        Ok(())
    }

    fn cancel_upgrade(env: Env) -> Result<(), Error> {
        let wasm_hash: BytesN<32> = Self::unschedule(&env, &ScheduledKey::Upgrade)?;

        let event = UpgradeCancelled { wasm_hash };
        event.publish(&env);

        Ok(())
    }

    fn get_scheduled_upgrade(env: &Env) -> Option<TimeLockedWasm> {
        let storage = env.storage().temporary();
        storage.get(&ScheduledKey::Upgrade)
    }

    fn schedule_admin_transfer(env: Env, new_admin: Address) -> Result<(), Error> {
        let admin = Self::schedule(&env, &ScheduledKey::Admin, new_admin)?;

        let event = AdminTransferInitiated { admin };
        event.publish(&env);

        Ok(())
    }

    fn cancel_admin_transfer(env: Env) -> Result<(), Error> {
        let admin: Address = Self::unschedule(&env, &ScheduledKey::Admin)?;

        let event = AdminTransferCancelled { admin };
        event.publish(&env);

        Ok(())
    }

    fn execute_admin_transfer(env: Env) -> Result<(), Error> {
        let admin: Address = Self::unschedule_if_valid(&env, &ScheduledKey::Admin)?;

        // also remove scheduled operations signed by previous admin
        let _ = Self::cancel_set_parameters(env.clone());
        let _ = Self::cancel_set_governance_params(env.clone());
        let _ = Self::cancel_upgrade(env.clone());

        Self::set_admin(&env, admin);

        Ok(())
    }

    fn get_scheduled_admin(env: &Env) -> Option<TimeLockedAddress> {
        let storage = env.storage().temporary();
        storage.get(&ScheduledKey::Admin)
    }
}

const fn parse_version(s: &str) -> u32 {
    match u32::from_str_radix(s, 10) {
        Ok(v) => v,
        Err(_) => panic!("invalid version number"),
    }
}

#[contractimpl]
impl stellar_oracle_shield_client::Governance for Contract {
    /// schedule a parameter change. The change can be applied after current timelock delay
    /// fails if a change is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_set_parameters(env: Env, parameters: Parameters) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_set_parameters(env, parameters)
    }

    /// execute a scheduled parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    fn execute_set_parameters(env: Env) -> Result<(), Error> {
        // note - no auth required, pending parameter has already been authorized
        Contract::execute_set_parameters(env)
    }

    /// cancel a scheduled parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    ///
    /// restricted to admin
    fn cancel_set_parameters(env: Env) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::cancel_set_parameters(env)
    }

    /// contract parameter set (active)
    fn get_parameters(env: Env) -> Parameters {
        // public - no auth
        Contract::get_parameters(&env)
    }

    /// scheduled contract parameter set if any
    fn get_scheduled_parameters(env: Env) -> Option<Parameters> {
        // public - no auth
        Contract::get_scheduled_parameters(&env).map(|t| t.value)
    }

    /// schedule max staleness for all pairs score
    /// simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_set_max_staleness(env, max_staleness)
    }

    /// schedule maximum deviation from the median for a fresh score to be considered agreeing
    /// simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_max_deviation(env: Env, max_deviation: u32) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_set_max_deviation(env, max_deviation)
    }

    /// set minimum number of fresh scores agreeing with the median score. Defaults to one.
    /// Zero disables the quorum constraint but still requires a fresh score.
    /// simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_quorum(env: Env, quorum: u32) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_set_quorum(env, quorum)
    }

    /// schedule add operator
    /// simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_add_operator(env: Env, operator: Address) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_add_operator(env, operator)
    }

    /// schedule remove operator - simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_remove_operator(env: Env, operator: Address) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_remove_operator(env, operator)
    }

    /// schedule set operators - simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_operators(env: Env, operators: Map<Address, ()>) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_set_operators(env, operators)
    }

    /// schedule a governance parameter change. The change can be applied after current timelock delay
    /// fails if a changed is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_set_governance_params(
        env: Env,
        parameters: GovernanceParameters,
    ) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_set_governance_params(env, parameters)
    }

    /// execute a scheduled governance parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    fn execute_set_governance_params(env: Env) -> Result<(), Error> {
        // note - no auth required, pending parameter has already been authorized
        Contract::execute_set_governance_params(env)
    }

    /// cancel a scheduled governance parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    ///
    /// restricted to admin
    fn cancel_set_governance_params(env: Env) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::cancel_set_governance_params(env)
    }

    /// contract governance parameter set (active)
    fn get_governance_params(env: Env) -> GovernanceParameters {
        // public - no auth
        Contract::get_governance_params(&env)
    }

    /// contract governance parameter constraints (contract immutable)
    fn get_governance_constraints(env: Env) -> GovernanceParametersConstraints {
        // public - no auth
        Contract::get_governance_parameters_constraints(&env)
    }

    /// scheduled contract governance parameter set if any
    fn get_scheduled_governance_params(env: Env) -> Option<GovernanceParameters> {
        // public - no auth
        Contract::get_scheduled_governance_params(&env).map(|t| t.value)
    }

    /// schedule an upgrade of the contract
    /// `new_wasm_hash` - hash of the new wasm
    /// fails if an upgrade is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_upgrade(env, new_wasm_hash)
    }

    /// execute a scheduled upgrade
    /// fails if timelock delay is not yet elapsed, no upgrade scheduled or scheduled upgrade expired
    fn execute_upgrade(env: Env) -> Result<(), Error> {
        // note - no auth required, pending action has already been authorized
        Contract::execute_upgrade(env)
    }

    /// cancel a scheduled upgrade
    /// fails if timelock delay is not yet elapsed, no upgrade scheduled or scheduled upgrade expired
    ///
    /// restricted to admin
    fn cancel_upgrade(env: Env) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::cancel_upgrade(env)
    }

    /// scheduled upgrade if any
    fn get_scheduled_upgrade(env: Env) -> Option<BytesN<32>> {
        // public - no auth
        Contract::get_scheduled_upgrade(&env).map(|t| t.value)
    }

    /// schedule an hand over administration privileges
    /// current administrator will keep administration privileges
    /// until new_admin has accepted them
    /// fails if a transfer is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_admin_transfer(env: Env, new_admin: Address) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::schedule_admin_transfer(env, new_admin)
    }

    /// accept administration privileges
    /// fails if timelock delay is not yet elapsed, no transfer scheduled or scheduled transfer expired
    ///
    /// restricted to *pending* admin
    fn execute_admin_transfer(env: Env) -> Result<(), Error> {
        // important - the execution must be triggered by new administrator
        // to ensure the address is valid
        let scheduled_admin = Contract::get_scheduled_admin(&env)
            .map(|t| t.value)
            .ok_or(Error::Expired)?;
        scheduled_admin.require_auth();
        Contract::execute_admin_transfer(env)
    }

    /// cancel pending hand over administration privileges
    /// fails if timelock delay is not yet elapsed, no transfer scheduled or scheduled transfer expired
    ///
    /// restricted to admin
    fn cancel_admin_transfer(env: Env) -> Result<(), Error> {
        let admin = Contract::get_admin(&env)?;
        admin.require_auth();
        Contract::cancel_admin_transfer(env)
    }

    /// scheduled admin transfer if any
    fn get_scheduled_admin(env: Env) -> Option<Address> {
        // public - no auth
        Contract::get_scheduled_admin(&env).map(|t| t.value)
    }
}

#[contractimpl]
impl stellar_oracle_shield_client::Scoring for Contract {
    /// set score of a pair in legacy single operator mode
    /// `operator` - operator generating the score
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    /// `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to operator
    fn set_score(
        env: Env,
        operator: Address,
        base: Address,
        quote: Address,
        score: u32,
    ) -> Result<(), Error> {
        Contract::require_operator_auth(&env, &operator)?;
        Contract::set_score(&env, operator, base, quote, score)
    }

    /// scores for multiple pairs
    /// `operator` - operator generating scores
    /// `scores` - vector of scores with the struct (base, quote, score)
    /// base - SAC address of an asset
    /// quote - SAC address of an asset
    /// score - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to operator
    fn set_scores(
        env: Env,
        operator: Address,
        scores: Vec<(Address, Address, u32)>,
    ) -> Result<(), Error> {
        Contract::require_operator_auth(&env, &operator)?;
        Contract::set_scores(env, operator, scores)
    }

    /// set score of a pair
    /// `operator` - operator generating the score
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    /// `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to operator
    fn set_score_from(
        env: Env,
        operator: Address,
        base: Address,
        quote: Address,
        score: u32,
    ) -> Result<(), Error> {
        Contract::require_operator_auth(&env, &operator)?;
        Contract::set_score(&env, operator, base, quote, score)
    }
}

#[contractimpl]
impl stellar_oracle_shield_client::Contract for Contract {
    /// get score of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    ///
    /// return the score
    /// fails if
    /// - pair is not covered
    /// - no fresh operator reports remain
    /// - median status consensus is below quorum
    fn get_score(env: Env, base: Address, quote: Address) -> Result<u32, Error> {
        Contract::get_score(env, base, quote)
    }

    /// get health status of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    ///
    /// return the score
    /// fails if
    /// - pair is not covered
    /// - no fresh operator reports remain
    /// - median status consensus is below quorum
    fn get_status(env: Env, base: Address, quote: Address) -> Result<Status, Error> {
        Contract::get_status(env, base, quote)
    }

    /// retrieve version of the contract
    ///
    /// returns the version (major, minor, patch)
    fn version() -> (u32, u32, u32) {
        Contract::version()
    }
}

mod tests;
