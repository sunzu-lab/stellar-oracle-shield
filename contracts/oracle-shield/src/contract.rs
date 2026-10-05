use {
    crate::score::{AggregatedScore, Score, TimestampedScore},
    soroban_sdk::{
        Address, BytesN, Env, Map, Vec, contract, contractevent, contractimpl, contractmeta,
        contracttype,
    },
    stellar_oracle_shield_client::{Error, Status},
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
    /// operator storage access key for legacy single operator mode
    Operator,
    /// operators storage access key for authorized operators
    Operators,
    /// staleness configuration access key
    MaxStaleness,
    /// maximum deviation from the median for a fresh score to be considered agreeing
    MaxDeviation,
    /// minimum number of fresh scores agreeing with the median score
    Quorum,
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

const DEFAULT_MAX_STALENESS_SECONDS: u64 = 3600;
const DEFAULT_MAX_DEVIATION: u32 = 5;
const DEFAULT_QUORUM: u32 = 1;

#[contractimpl]
impl Contract {
    /// initialze contract
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
        env.storage().instance().set(
            &DataKey::MaxStaleness,
            &max_staleness.unwrap_or(DEFAULT_MAX_STALENESS_SECONDS),
        );
        env.storage().instance().set(
            &DataKey::MaxDeviation,
            &max_deviation.unwrap_or(DEFAULT_MAX_DEVIATION),
        );
        env.storage()
            .instance()
            .set(&DataKey::Quorum, &quorum.unwrap_or(DEFAULT_QUORUM));
        if let Some(operators) = operators {
            Self::set_operators_unchecked(&env, operators).expect("failed to set operators");
        }
    }

    fn version() -> (u32, u32, u32) {
        VERSION
    }

    fn set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::MaxStaleness, &max_staleness);
        Ok(())
    }

    fn get_max_staleness(env: &Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MaxStaleness)
            .unwrap_or(DEFAULT_MAX_STALENESS_SECONDS)
    }

    fn set_max_deviation(env: Env, max_deviation: u32) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::MaxDeviation, &max_deviation);
        Ok(())
    }

    fn get_max_deviation(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::MaxDeviation)
            .unwrap_or(DEFAULT_MAX_DEVIATION)
    }

    fn set_quorum(env: Env, quorum: u32) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Quorum, &quorum);
        Ok(())
    }

    fn get_quorum(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Quorum)
            .unwrap_or(DEFAULT_QUORUM)
    }

    fn set_operator_key(env: Env, operator_key: Address) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        let mut operators = Map::new(&env);
        operators.set(operator_key, ());
        Self::set_inner_operators(&env, &operators);
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
        storage.remove(&DataKey::Operator);
    }

    fn get_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::MissingAdmin)
    }

    fn get_operators(env: &Env) -> Map<Address, ()> {
        let storage = env.storage().instance();

        let default_operators = || -> Map<Address, ()> {
            let mut operators = Map::new(env);

            // backward compatibility with legacy single operator
            if let Some(operator) = storage.get(&DataKey::Operator) {
                operators.set(operator, ());
            }

            operators
        };

        storage
            .get(&DataKey::Operators)
            .unwrap_or_else(default_operators)
    }

    fn set_score(env: Env, base: Address, quote: Address, score: u32) -> Result<(), Error> {
        let operators = Self::get_operators(&env);
        if operators.len() != 1 {
            return Err(Error::UnauthorizedOperator);
        }

        let (operator, _) = operators.iter().next().unwrap();
        operator.require_auth();

        Self::set_score_from_unchecked(env, operator, base, quote, score)
    }

    fn set_score_from(
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

        Self::set_score_from_unchecked(env, operator, base, quote, score)
    }

    fn set_score_from_unchecked(
        env: Env,
        operator: Address,
        base: Address,
        quote: Address,
        score: u32,
    ) -> Result<(), Error> {
        let pair = Pair(base, quote);
        let old_score = Self::get_inner_score(&env, &pair);
        let ope_score = TimestampedScore::new(score, env.ledger().timestamp())?;
        let key = OperatorScoreKey(pair.clone(), operator);
        env.storage().temporary().set(&key, &ope_score);
        let new_score = Self::get_inner_score(&env, &pair);

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
                .publish(&env);
            }

            if status_changed {
                StatusChange {
                    base: pair.0.clone(),
                    quote: pair.1.clone(),
                    status: score.status(),
                }
                .publish(&env);
            }
        }
        Ok(())
    }

    fn get_inner_score(env: &Env, pair: &Pair) -> Result<AggregatedScore, Error> {
        let max_staleness = Self::get_max_staleness(env);
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
                if age > max_staleness {
                    continue;
                }
                oldest_ts = oldest_ts.min(ts_score.ts);
                let mut index = 0;
                while index < values.len() && values.get_unchecked(index) <= ts_score.score.score {
                    index += 1;
                }
                values.insert(index, ts_score.score.score);
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
        let max_deviation = Self::get_max_deviation(env);
        let mut consensus = 0_u32;
        for value in values.iter() {
            if value.abs_diff(median) <= max_deviation {
                consensus += 1;
            }
        }

        let quorum = Self::get_quorum(env);
        if consensus < quorum {
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

        let storage = env.storage().temporary();
        storage.set(&DataKey::PendingAdmin, &new_admin);
    }

    /// accept administration privileges
    ///
    /// restricted to pending admin
    pub fn accept_admin(env: Env) {
        let storage = env.storage().temporary();
        if let Some(pending_admin) = storage.get::<_, Address>(&DataKey::PendingAdmin) {
            pending_admin.require_auth();
            storage.remove(&DataKey::PendingAdmin);

            let storage = env.storage().instance();
            storage.set(&DataKey::Admin, &pending_admin);
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

    /// set operator address in legacy single operator mode
    /// `operator_key` - Address
    ///
    /// restricted to admin
    fn set_operator_key(env: Env, operator_key: Address) -> Result<(), Error> {
        Contract::set_operator_key(env, operator_key)
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
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    /// `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to operator
    fn set_score(env: Env, base: Address, quote: Address, score: u32) -> Result<(), Error> {
        Contract::set_score(env, base, quote, score)
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
        Contract::set_score_from(env, operator, base, quote, score)
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
