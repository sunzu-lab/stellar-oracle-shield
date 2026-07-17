use {
    crate::{error::Error, score::Score, status::Status},
    soroban_sdk::{
        contract, contractevent, contractimpl, contractmeta, contracttype, Address, Env,
    },
};

contractmeta!(key = "Description", val = "sunzu lab oracle shield");

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
enum DataKey {
    Admin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Pair(Address, Address);

/// oracle shield main contract
#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// initialze contract
    /// set administator address
    pub fn __constructor(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
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
    pub fn set_score(env: Env, base: Address, quote: Address, score: i32) -> Result<(), Error> {
        let admin = Self::get_admin(&env).ok_or(Error::MissingAdmin)?;
        admin.require_auth();
        let pair = Pair(base, quote);
        let old_score = Self::get_inner_score(&env, &pair);
        let score = Score::new(score)?;
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
        // todo - if stale - return StaleInput error
        env.storage()
            .temporary()
            .get(&pair)
            .ok_or(Error::PairNotCovered)
    }

    /// get score of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    ///
    /// return the score
    /// fails if
    /// - pair is not covered
    /// - input for pair is stale (unreliable score)
    pub fn get_score(env: Env, base: Address, quote: Address) -> Result<i32, Error> {
        let score = Self::get_inner_score(&env, &Pair(base, quote))?;
        Ok(score.get())
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
}

#[contractevent(
    topics = ["HEALTH_CHANGED"],
    data_format = "single-value"
)]
pub struct StatusChange {
    #[topic]
    pub base: Address,
    #[topic]
    pub quote: Address,
    pub status: Status,
}

mod tests;
