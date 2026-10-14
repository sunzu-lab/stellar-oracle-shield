use {
    crate::{error::Error, status::Status},
    soroban_sdk::{Address, Env, Vec, contractclient},
};

/// oracle shield main contract trait
#[contractclient(name = "ContractClient")]
pub trait Contract {
    /// set max staleness for all pairs score
    /// `max staleness` - u64 seconds
    ///
    /// restricted to admin
    fn set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error>;

    /// set maximum deviation from the median for a fresh score to be considered agreeing
    ///
    /// restricted to admin
    fn set_max_deviation(env: Env, max_deviation: u32) -> Result<(), Error>;

    /// set minimum number of fresh scores agreeing with the median score. Defaults to one.
    /// Zero disables the quorum constraint but still requires a fresh score.
    /// restricted to admin
    fn set_quorum(env: Env, quorum: u32) -> Result<(), Error>;

    /// set operator address in legacy single operator mode
    /// `operator_key` - Address
    ///
    /// restricted to admin
    #[deprecated = "use set_operators with 1-element vector instead"]
    fn set_operator_key(env: Env, operator_key: Address) -> Result<(), Error>;

    /// set operator addresses
    /// `operators` - Addresses
    ///
    /// restricted to admin
    fn set_operators(env: Env, operators: Vec<Address>) -> Result<(), Error>;

    /// add operator
    /// `operator` - Address
    ///
    /// restricted to admin
    fn add_operator(env: Env, operator: Address) -> Result<(), Error>;

    /// remove operator
    /// `operator` - Address
    ///
    /// restricted to admin
    fn remove_operator(env: Env, operator: Address) -> Result<(), Error>;

    /// set score of a pair in legacy single operator mode
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    /// `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to operator
    #[deprecated = "use set_score_from instead"]
    fn set_score(env: Env, base: Address, quote: Address, score: u32) -> Result<(), Error>;

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
    ) -> Result<(), Error>;

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
