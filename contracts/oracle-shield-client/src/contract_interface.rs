use {
    crate::{
        error::Error, governance_parameters::GovernanceParameters,
        governance_parameters_constraints::GovernanceParametersConstraints, parameters::Parameters,
        status::Status,
    },
    soroban_sdk::{Address, BytesN, Env, Map, Vec, contractclient},
};

/// oracle shield governance contract trait
/// scope: admin
#[contractclient(name = "GovernanceClient")]
pub trait Governance {
    /// schedule a parameter change. The change can be applied after current timelock delay
    /// fails if a change is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_set_parameters(env: Env, parameters: Parameters) -> Result<(), Error>;

    /// execute a scheduled parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    fn execute_set_parameters(env: Env) -> Result<(), Error>;

    /// cancel a scheduled parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    ///
    /// restricted to admin
    fn cancel_set_parameters(env: Env) -> Result<(), Error>;

    /// contract parameter set (active)
    fn get_parameters(env: Env) -> Parameters;

    /// scheduled contract parameter set if any
    fn get_scheduled_parameters(env: Env) -> Option<Parameters>;

    /// schedule max staleness for all pairs score
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_max_staleness(env: Env, max_staleness: u64) -> Result<(), Error>;

    /// schedule maximum deviation from the median for a fresh score to be considered agreeing
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_max_deviation(env: Env, max_deviation: u32) -> Result<(), Error>;

    /// set minimum number of fresh scores agreeing with the median score. Defaults to one.
    /// Zero disables the quorum constraint but still requires a fresh score.
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_quorum(env: Env, quorum: u32) -> Result<(), Error>;

    /// schedule add operator
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_add_operator(env: Env, operator: Address) -> Result<(), Error>;

    /// schedule remove operator - simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_remove_operator(env: Env, operator: Address) -> Result<(), Error>;

    /// schedule set operators - simple helper over schedule_set_parameters
    /// simple helper over `schedule_set_parameters` using current parameters
    ///
    /// restricted to admin
    fn schedule_set_operators(env: Env, operators: Map<Address, ()>) -> Result<(), Error>;

    /// schedule a governance parameter change. The change can be applied after current timelock delay
    /// fails if a changed is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_set_governance_params(
        env: Env,
        parameters: GovernanceParameters,
    ) -> Result<(), Error>;

    /// execute a scheduled governance parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    fn execute_set_governance_params(env: Env) -> Result<(), Error>;

    /// cancel a scheduled governance parameter change
    /// fails if timelock delay is not yet elapsed, no change scheduled or scheduled changed expired
    ///
    /// restricted to admin
    fn cancel_set_governance_params(env: Env) -> Result<(), Error>;

    /// contract governance parameter set (active)
    fn get_governance_params(env: Env) -> GovernanceParameters;

    /// contract governance parameter constraints (contract immutable)
    fn get_governance_constraints(env: Env) -> GovernanceParametersConstraints;

    /// scheduled contract governance parameter set if any
    fn get_scheduled_governance_params(env: Env) -> Option<GovernanceParameters>;

    /// schedule an upgrade of the contract
    /// `new_wasm_hash` - hash of the new wasm
    /// fails if an upgrade is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error>;

    /// execute a scheduled upgrade
    /// fails if timelock delay is not yet elapsed, no upgrade scheduled or scheduled upgrade expired
    fn execute_upgrade(env: Env) -> Result<(), Error>;

    /// cancel a scheduled upgrade
    /// fails if timelock delay is not yet elapsed, no upgrade scheduled or scheduled upgrade expired
    ///
    /// restricted to admin
    fn cancel_upgrade(env: Env) -> Result<(), Error>;

    /// scheduled upgrade if any
    fn get_scheduled_upgrade(env: Env) -> Option<BytesN<32>>;

    /// schedule an hand over administration privileges
    /// current administrator will keep administration privileges
    /// until new_admin has accepted them
    /// fails if a transfer is already pending. Is has to be cancelled first.
    ///
    /// restricted to admin
    fn schedule_admin_transfer(env: Env, new_admin: Address) -> Result<(), Error>;

    /// accept administration privileges
    /// fails if timelock delay is not yet elapsed, no transfer scheduled or scheduled transfer expired
    ///
    /// restricted to *pending* admin
    fn execute_admin_transfer(env: Env) -> Result<(), Error>;

    /// cancel pending hand over administration privileges
    /// fails if timelock delay is not yet elapsed, no transfer scheduled or scheduled transfer expired
    ///
    /// restricted to admin
    fn cancel_admin_transfer(env: Env) -> Result<(), Error>;

    /// scheduled admin transfer if any
    fn get_scheduled_admin(env: Env) -> Option<Address>;
}

/// oracle shield scorer contract trait
/// scope: scorer operator
#[contractclient(name = "ScoringClient")]
pub trait Scoring {
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
    ) -> Result<(), Error>;

    /// scores for multiple pairs
    /// `operator` - operator generating scores
    /// `scores` - vector of scores with the struct (base, quote, score)
    /// base - SAC address of an asset
    /// quote - SAC address of an asset
    /// score - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    ///
    /// restricted to operator
    fn set_scores(
        env: Env,
        operator: Address,
        scores: Vec<(Address, Address, u32)>,
    ) -> Result<(), Error>;

    /// set score of a pair
    /// `operator` - operator generating the score
    /// `base` - SAC address of an asset
    /// `quote` - SAC address of an asset
    /// `score` - [0-100] scoring. 0 the more unsafe, 100 the healthier
    ///
    /// restricted to operator
    #[deprecated = "use set_score instead"]
    fn set_score_from(
        env: Env,
        operator: Address,
        base: Address,
        quote: Address,
        score: u32,
    ) -> Result<(), Error>;
}

/// oracle shield user contract trait
/// scope: client contract
#[contractclient(name = "ContractClient")]
pub trait Contract {
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
