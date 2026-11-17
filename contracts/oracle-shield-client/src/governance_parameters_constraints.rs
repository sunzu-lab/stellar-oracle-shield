use {
    crate::{Error, GovernanceParameters},
    soroban_sdk::{Env, contracttype},
};

const ONE_MINUTE_SECONDS: u64 = 60;
const ONE_HOUR_SECONDS: u64 = 60 * ONE_MINUTE_SECONDS;
const ONE_DAY_SECONDS: u64 = 24 * ONE_HOUR_SECONDS;

const DEFAULT_MIN_TIMELOCK_DELAY_SECONDS: u64 = ONE_DAY_SECONDS;
const DEFAULT_MAX_TIMELOCK_DELAY_SECONDS: u64 = 7 * ONE_DAY_SECONDS;
const MAX_MAX_TIMELOCK_DELAY_SECONDS: u64 = 30 * ONE_DAY_SECONDS;

const _: () = {
    assert!(DEFAULT_MIN_TIMELOCK_DELAY_SECONDS <= DEFAULT_MAX_TIMELOCK_DELAY_SECONDS);
    assert!(DEFAULT_MAX_TIMELOCK_DELAY_SECONDS <= MAX_MAX_TIMELOCK_DELAY_SECONDS);
};

/// governance parameters constraints
/// should be immutable (defined only once) for contract lifetime
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceParametersConstraints {
    /// maximum possible timelock_delay_seconds
    pub max_timelock_delay_seconds: u64,
    /// minimum possible timelock_delay_seconds
    pub min_timelock_delay_seconds: u64,
}

impl Default for GovernanceParametersConstraints {
    fn default() -> Self {
        Self {
            min_timelock_delay_seconds: DEFAULT_MIN_TIMELOCK_DELAY_SECONDS,
            max_timelock_delay_seconds: DEFAULT_MAX_TIMELOCK_DELAY_SECONDS,
        }
    }
}

impl GovernanceParametersConstraints {
    pub fn is_valid(&self, env: &Env) -> Result<(), Error> {
        if self.min_timelock_delay_seconds > self.max_timelock_delay_seconds {
            return Err(Error::InvalidParam);
        }

        if self.max_timelock_delay_seconds > MAX_MAX_TIMELOCK_DELAY_SECONDS {
            return Err(Error::InvalidParam);
        }

        let governance_parameters = GovernanceParameters {
            timelock_delay_seconds: self.max_timelock_delay_seconds,
            ..Default::default()
        };
        if governance_parameters.required_ttl() > env.storage().max_ttl() {
            return Err(Error::InvalidParam);
        }

        Ok(())
    }

    pub fn timelock_range(&self) -> core::ops::RangeInclusive<u64> {
        self.min_timelock_delay_seconds..=self.max_timelock_delay_seconds
    }
}
