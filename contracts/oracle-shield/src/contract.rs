use {
    crate::score::{AggregatedScore, Score, TimestampedScore},
    soroban_sdk::{
        Address, BytesN, Env, Map, Vec, contract, contractevent, contractimpl, contractmeta,
        contracttype,
    },
    stellar_oracle_shield_client::{Error, Parameters, Status},
};

contractmeta!(key = "name", val = env!("CARGO_PKG_NAME"));
contractmeta!(key = "version", val = env!("CARGO_PKG_VERSION"));
contractmeta!(key = "description", val = env!("CARGO_PKG_DESCRIPTION"));
contractmeta!(key = "license", val = env!("CARGO_PKG_LICENSE"));

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
enum DataKey {
    /// administrator storage access key
    Admin,
    /// pending administrator storage access key
    PendingAdmin,
    /// operators storage access key for authorized operators
    Operators,
    /// contract parameter set
    Parameters,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Pair(Address, Address);

/// latest report by an operator for an ordered pair
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct OperatorScoreKey(Pair, Address);

/// oracle shield main contract
#[contract]
pub struct Contract;

const VERSION: (u32, u32, u32) = (
    parse_version(env!("CARGO_PKG_VERSION_MAJOR")),
    parse_version(env!("CARGO_PKG_VERSION_MINOR")),
    parse_version(env!("CARGO_PKG_VERSION_PATCH")),
);

const ADMIN_TRANSFER_DELAY_SECONDS: u64 = 86_400;
const ADMIN_TRANSFER_TTL: u32 = 120_960;

#[contractimpl]
impl Contract {
    /// initialize contract
    /// set administator address
    pub fn __constructor(
        env: Env,
        admin: Address,
        max_staleness: Option<u64>,
        max_deviation: Option<u32>,
        quorum: Option<u32>,
        operators: Option<Vec<Address>>,
    ) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        let default_parameters = Parameters::default();
        let parameters = Parameters {
            max_staleness: max_staleness.unwrap_or(default_parameters.max_staleness),
            max_deviation: max_deviation.unwrap_or(default_parameters.max_deviation),
            quorum: quorum.unwrap_or(default_parameters.quorum),
        };
        Self::set_parameters_unchecked(&env, &parameters);
        if let Some(operators) = operators {
            Self::set_operators_unchecked(&env, operators).expect("failed to set operators");
        }
    }

    fn version() -> (u32, u32, u32) {
        VERSION
    }

    fn set_parameters_unchecked(env: &Env, parameters: &Parameters) {
        env.storage()
            .instance()
            .set(&DataKey::Parameters, &parameters);

        ParametersChange {
            parameters: *parameters,
        }
        .publish(env);
    }

    fn get_parameters(env: &Env) -> Parameters {
        env.storage()
            .instance()
            .get(&DataKey::Parameters)
            .unwrap_or_default()
    }

    fn set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        let mut parameters = Self::get_parameters(&env);
        parameters.max_staleness = max_staleness;
        Self::set_parameters_unchecked(&env, &parameters);
        Ok(())
    }

    fn set_max_deviation(env: Env, max_deviation: u32) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        let mut parameters = Self::get_parameters(&env);
        parameters.max_deviation = max_deviation;
        Self::set_parameters_unchecked(&env, &parameters);
        Ok(())
    }

    fn set_quorum(env: Env, quorum: u32) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        let mut parameters = Self::get_parameters(&env);
        parameters.quorum = quorum;
        Self::set_parameters_unchecked(&env, &parameters);
        Ok(())
    }

    fn add_operator(env: Env, operator: Address) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();

        let mut operators = Self::get_operators(&env);
        operators.set(operator, ());
        Self::set_inner_operators(&env, &operators);

        Ok(())
    }

    fn remove_operator(env: Env, operator: Address) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();

        let mut operators = Self::get_operators(&env);
        operators.remove(operator);
        Self::set_inner_operators(&env, &operators);

        Ok(())
    }

    fn set_operators(env: Env, operators: Vec<Address>) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();

        Self::set_operators_unchecked(&env, operators)
    }

    fn set_operators_unchecked(env: &Env, operators: Vec<Address>) -> Result<(), Error> {
        let mut op_map: Map<Address, ()> = Map::new(env);

        for operator in operators.iter() {
            op_map.set(operator, ());
        }

        Self::set_inner_operators(env, &op_map);

        Ok(())
    }

    fn set_inner_operators(env: &Env, operators: &Map<Address, ()>) {
        let storage = env.storage().instance();
        storage.set(&DataKey::Operators, operators);
    }

    fn get_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::MissingAdmin)
    }

    fn get_operators(env: &Env) -> Map<Address, ()> {
        let default = || Map::new(env);
        let storage = env.storage().instance();
        storage.get(&DataKey::Operators).unwrap_or_else(default)
    }

    fn set_scores(
        env: Env,
        operator: Address,
        scores: Vec<(Address, Address, u32)>,
    ) -> Result<(), Error> {
        let operators = Self::get_operators(&env);
        if !operators.contains_key(operator.clone()) {
            return Err(Error::UnauthorizedOperator);
        }
        operator.require_auth();

        for (base, quote, score) in scores {
            Self::set_score_unchecked(&env, operator.clone(), base, quote, score)?;
        }
        Ok(())
    }

    fn set_score(
        env: Env,
        operator: Address,
        base: Address,
        quote: Address,
        score: u32,
    ) -> Result<(), Error> {
        let operators = Self::get_operators(&env);
        if !operators.contains_key(operator.clone()) {
            return Err(Error::UnauthorizedOperator);
        }
        operator.require_auth();

        Self::set_score_unchecked(&env, operator, base, quote, score)
    }

    fn set_score_unchecked(
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

    /// upgrade the contract with the new one
    /// `new_wasm_hash` - hash of the new wasm
    ///
    /// restricted to admin
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin = Self::get_admin(&env).unwrap();
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    /// hand over administration privileges
    /// current administrator will kill administration privileges
    /// until new_admin has accepted them
    ///
    /// restricted to admin
    pub fn hand_over_admin(env: Env, new_admin: Address) {
        let admin = Self::get_admin(&env).unwrap();
        admin.require_auth();

        let now = env.ledger().timestamp();
        let valid_after = now + ADMIN_TRANSFER_DELAY_SECONDS;
        let candidate = (new_admin, valid_after);
        let storage = env.storage().temporary();
        storage.set(&DataKey::PendingAdmin, &candidate);
        storage.extend_ttl(
            &DataKey::PendingAdmin,
            ADMIN_TRANSFER_TTL,
            ADMIN_TRANSFER_TTL,
        );

        let event = AdminTransferInitiated { admin, candidate };
        event.publish(&env);
    }

    /// cancel pending hand over administration privileges
    ///
    /// restricted to admin
    pub fn cancel_hand_over_admin(env: Env) {
        let admin = Self::get_admin(&env).unwrap();
        admin.require_auth();

        let storage = env.storage().temporary();
        if let Some(candidate) = storage.get::<_, (Address, u64)>(&DataKey::PendingAdmin) {
            storage.remove(&DataKey::PendingAdmin);

            let event = AdminTransferCancelled { admin, candidate };
            event.publish(&env);
        }
    }

    /// accept administration privileges
    ///
    /// restricted to pending admin
    pub fn accept_admin(env: Env) -> Result<(), Error> {
        let storage = env.storage().temporary();
        if let Some((pending_admin, valid_after)) =
            storage.get::<_, (Address, u64)>(&DataKey::PendingAdmin)
        {
            pending_admin.require_auth();

            if env.ledger().timestamp() < valid_after {
                return Err(Error::TimeLocked);
            }

            storage.remove(&DataKey::PendingAdmin);

            let old_admin = Self::get_admin(&env).unwrap();
            let new_admin = pending_admin;
            let storage = env.storage().instance();
            storage.set(&DataKey::Admin, &new_admin);
            let event = AdminTransfered {
                old_admin,
                new_admin,
            };
            event.publish(&env);
            Ok(())
        } else {
            Err(Error::Expired)
        }
    }
}

#[contractevent(data_format = "single-value")]
pub struct ScoreChange {
    #[topic]
    pub base: Address,
    #[topic]
    pub quote: Address,
    pub score: u32,
}

#[contractevent(data_format = "single-value")]
pub struct StatusChange {
    #[topic]
    pub base: Address,
    #[topic]
    pub quote: Address,
    pub status: Status,
}

#[contractevent(data_format = "single-value")]
pub struct ParametersChange {
    pub parameters: Parameters,
}

#[contractevent(data_format = "single-value")]
pub struct AdminTransferInitiated {
    #[topic]
    pub admin: Address,
    pub candidate: (Address, u64),
}

#[contractevent(data_format = "single-value")]
pub struct AdminTransferCancelled {
    #[topic]
    pub admin: Address,
    pub candidate: (Address, u64),
}

#[contractevent(data_format = "single-value")]
pub struct AdminTransfered {
    #[topic]
    pub old_admin: Address,
    pub new_admin: Address,
}

const fn parse_version(s: &str) -> u32 {
    match u32::from_str_radix(s, 10) {
        Ok(v) => v,
        Err(_) => panic!("invalid version number"),
    }
}

#[contractimpl]
impl stellar_oracle_shield_client::Contract for Contract {
    /// set max staleness for all pairs score
    /// `max staleness` - u64 seconds
    ///
    /// restricted to admin
    fn set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error> {
        Contract::set_max_staleness(env, max_staleness)
    }

    /// set maximum deviation from the median for a fresh score to be considered agreeing
    ///
    /// restricted to admin
    fn set_max_deviation(env: Env, max_deviation: u32) -> Result<(), Error> {
        Contract::set_max_deviation(env, max_deviation)
    }

    /// set minimum number of fresh scores agreeing with the median score. Defaults to one.
    /// Zero disables the quorum constraint but still requires a fresh score.
    /// restricted to admin
    fn set_quorum(env: Env, quorum: u32) -> Result<(), Error> {
        Contract::set_quorum(env, quorum)
    }

    /// contract parameter set
    fn get_parameters(env: Env) -> Result<Parameters, Error> {
        Ok(Contract::get_parameters(&env))
    }

    /// set operator addresses
    /// `operators` - Addresses
    ///
    /// restricted to admin
    fn set_operators(env: Env, operators: Vec<Address>) -> Result<(), Error> {
        Contract::set_operators(env, operators)
    }

    /// add operator
    /// `operator` - operator address
    ///
    /// restricted to admin
    fn add_operator(env: Env, operator: Address) -> Result<(), Error> {
        Contract::add_operator(env, operator)
    }

    /// remove operator
    /// `operator` - operator address
    ///
    /// restricted to admin
    fn remove_operator(env: Env, operator: Address) -> Result<(), Error> {
        Contract::remove_operator(env, operator)
    }

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
        Contract::set_score(env, operator, base, quote, score)
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
        Contract::set_score(env, operator, base, quote, score)
    }

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
