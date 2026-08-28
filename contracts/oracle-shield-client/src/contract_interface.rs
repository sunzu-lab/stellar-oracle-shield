use {
    crate::{error::Error, status::Status},
    soroban_sdk::{Address, Env, contractclient},
};

/// oracle shield main contract trait
#[contractclient(name = "ContractClient")]
pub trait Contract {
    /// set max staleness for all pairs score
    /// `max staleness` - u64 seconds
    ///
    /// restricted to admin
    fn set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error>;

    /// set operator address
    /// `operator_key` - Address
    ///
    /// restricted to admin
    fn set_operator_key(env: Env, operator_key: Address) -> Result<(), Error>;

    /// set score of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    /// `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to operator
    fn set_score(env: Env, base: Address, quote: Address, score: u32) -> Result<(), Error>;

    /// get score of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    ///
    /// return the score
    /// fails if
    /// - pair is not covered
    /// - input for pair is stale (unreliable score)
    fn get_score(env: Env, base: Address, quote: Address) -> Result<u32, Error>;

    /// get health status of a pair
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    ///
    /// return the score
    /// fails if
    /// - pair is not covered
    /// - input for pair is stale (unreliable score)
    fn get_status(env: Env, base: Address, quote: Address) -> Result<Status, Error>;

    /// retrieve version of the contract
    ///
    /// returns2 the version (major, minor, patch)
    fn version() -> (u32, u32, u32);
}
