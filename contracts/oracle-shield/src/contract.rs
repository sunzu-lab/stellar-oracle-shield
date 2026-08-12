use {
    crate::{error::Error, score::Score, status::Status},
    soroban_sdk::{
        Address, BytesN, Env, contract, contractevent, contractimpl, contractmeta, contracttype,
    },
};

contractmeta!(key = "name", val = env!("CARGO_PKG_NAME"));
contractmeta!(key = "version", val = env!("CARGO_PKG_VERSION"));
contractmeta!(key = "description", val = env!("CARGO_PKG_DESCRIPTION"));
contractmeta!(key = "license", val = env!("CARGO_PKG_LICENSE"));

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
enum DataKey {
    Admin,
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
    pub fn __constructor(env: Env, admin: Address, max_staleness: Option<u64>) {
        const DEFAULT_MAX_STALENESS_SECONDS: u64 = 3600;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(
            &DataKey::MaxStaleness,
            &max_staleness.unwrap_or(DEFAULT_MAX_STALENESS_SECONDS),
        );
    }

    /// retrieve version of the contract
    ///
    /// return the version
    pub fn version() -> (u32, u32, u32) {
        VERSION
    }

    /// set max staleness for all pairs score
    /// `max staleness` - u64 seconds
    ///
    /// restricted to admin
    pub fn set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error> {
        let admin = Self::get_admin(&env).ok_or(Error::MissingAdmin)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::MaxStaleness, &max_staleness);
        Ok(())
    }

    fn get_admin(env: &Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// set score of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    /// `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to admin
    pub fn set_score(env: Env, base: Address, quote: Address, score: u32) -> Result<(), Error> {
        let admin = Self::get_admin(&env).ok_or(Error::MissingAdmin)?;
        admin.require_auth();

        let pair = Pair(base, quote);
        let old_score = Self::get_inner_score(&env, &pair);
        let score = Score::new(score, env.ledger().timestamp())?;
        env.storage().temporary().set(&pair, &score);

        if let Ok(old_score) = old_score {
            if old_score.status() != score.status() {
                StatusChange {
                    base: pair.0,
                    quote: pair.1,
                    status: score.into(),
                }
                .publish(&env)
            }
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

    /// get score of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    ///
    /// return the score
    /// fails if
    /// - pair is not covered
    /// - input for pair is stale (unreliable score)
    pub fn get_score(env: Env, base: Address, quote: Address) -> Result<u32, Error> {
        let score = Self::get_inner_score(&env, &Pair(base, quote))?;
        Ok(score.score)
    }

    /// get health status of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    ///
    /// return the score
    /// fails if
    /// - pair is not covered
    /// - input for pair is stale (unreliable score)
    pub fn get_status(env: Env, base: Address, quote: Address) -> Result<Status, Error> {
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

mod tests;
