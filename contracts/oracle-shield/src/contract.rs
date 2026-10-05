use {
    crate::score::Score,
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
    /// operator storage access key for legacy single operator mode
    Operator,
    /// operators storage access key for authorized operators
    Operators,
    /// staleness configuration access key
    MaxStaleness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Pair(Address, Address);

/// oracle shield main contract
#[contract]
pub struct Contract;

const VERSION: (u32, u32, u32) = (
    parse_version(env!("CARGO_PKG_VERSION_MAJOR")),
    parse_version(env!("CARGO_PKG_VERSION_MINOR")),
    parse_version(env!("CARGO_PKG_VERSION_PATCH")),
);

#[contractimpl]
impl Contract {
    /// initialze contract
    /// set administator address
    pub fn __constructor(
        env: Env,
        admin: Address,
        max_staleness: Option<u64>,
        operators: Option<Vec<Address>>,
    ) {
        const DEFAULT_MAX_STALENESS_SECONDS: u64 = 3600;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(
            &DataKey::MaxStaleness,
            &max_staleness.unwrap_or(DEFAULT_MAX_STALENESS_SECONDS),
        );
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
        _operator: Address,
        base: Address,
        quote: Address,
        score: u32,
    ) -> Result<(), Error> {
        let pair = Pair(base, quote);
        let old_score = Self::get_inner_score(&env, &pair);
        let score = Score::new(score, env.ledger().timestamp())?;
        env.storage().temporary().set(&pair, &score);

        if let Ok(old_score) = old_score
            && old_score.status() != score.status()
        {
            StatusChange {
                base: pair.0,
                quote: pair.1,
                status: score.into(),
            }
            .publish(&env)
        }
        Ok(())
    }

    fn get_inner_score(env: &Env, pair: &Pair) -> Result<Score, Error> {
        env.storage()
            .temporary()
            .get(&pair)
            .ok_or(Error::PairNotCovered)
            .and_then(|score: Score| {
                let max_staleness = env
                    .storage()
                    .instance()
                    .get(&DataKey::MaxStaleness)
                    .ok_or(Error::NoMaxStalenessSet)?;
                if env.ledger().timestamp() - score.ts > max_staleness {
                    return Err(Error::StaleInput);
                }
                Ok(score)
            })
    }

    fn get_score(env: Env, base: Address, quote: Address) -> Result<u32, Error> {
        let score = Self::get_inner_score(&env, &Pair(base, quote))?;
        Ok(score.score)
    }

    fn get_status(env: Env, base: Address, quote: Address) -> Result<Status, Error> {
        let score = Self::get_inner_score(&env, &Pair(base, quote))?;
        Ok(score.into())
    }

    /// upgrade the contract with the new one
    /// `new_wasm_hash` - hash of the new wasm
    ///
    /// restricted to admin
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
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
    /// - input for pair is stale (unreliable score)
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
    /// - input for pair is stale (unreliable score)
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
