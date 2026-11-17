use {
    crate::{Error, GovernanceParametersConstraints},
    soroban_sdk::{Env, contracttype},
};

const ONE_MINUTE_SECONDS: u64 = 60;
const ONE_HOUR_SECONDS: u64 = 60 * ONE_MINUTE_SECONDS;
const ONE_DAY_SECONDS: u64 = 24 * ONE_HOUR_SECONDS;

const DEFAULT_TIMELOCK_DELAY_SECONDS: u64 = 2 * ONE_DAY_SECONDS;
const DEFAULT_EXECUTION_WINDOW_SECONDS: u64 = 2 * ONE_DAY_SECONDS;
// don't lock yourself out
const MIN_EXECUTION_WINDOW_SECONDS: u64 = ONE_HOUR_SECONDS;
const LEDGER_PERIOD_SECONDS: u64 = 5;

/// oracle shield governance parameters
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GovernanceParameters {
    /// minimum delay in seconds before a scheduled critical change can be executed
    pub timelock_delay_seconds: u64,
    /// time a scheduled operation is valid *after* time lock period
    pub execution_window_seconds: u64,
}

impl Default for GovernanceParameters {
    fn default() -> Self {
        Self {
            timelock_delay_seconds: DEFAULT_TIMELOCK_DELAY_SECONDS,
            execution_window_seconds: DEFAULT_EXECUTION_WINDOW_SECONDS,
        }
    }
}

impl GovernanceParameters {
    /// approximation of storage ttl required to outlive timelock delay + execution window
    pub fn required_ttl(&self) -> u32 {
        ((self.timelock_delay_seconds + self.execution_window_seconds) / LEDGER_PERIOD_SECONDS)
            .try_into()
            .unwrap_or(u32::MAX)
    }

    pub fn is_valid(
        &self,
        env: &Env,
        governance_parameters_constraints: &GovernanceParametersConstraints,
    ) -> Result<(), Error> {
        if !governance_parameters_constraints
            .timelock_range()
            .contains(&self.timelock_delay_seconds)
        {
            return Err(Error::InvalidParam);
        }

        if self.required_ttl() > env.storage().max_ttl() {
            return Err(Error::InvalidParam);
        }

        if self.execution_window_seconds < MIN_EXECUTION_WINDOW_SECONDS {
            return Err(Error::InvalidParam);
        }

        Ok(())
    }
}
