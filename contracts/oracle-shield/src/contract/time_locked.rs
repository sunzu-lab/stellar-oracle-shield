use {
    soroban_sdk::{Address, BytesN, Env, IntoVal, TryFromVal, Val, contracttype},
    stellar_oracle_shield_client::{GovernanceParameters, Parameters},
};

// could be simpler if #[contracttype] supported generics

pub trait IntoTimeLocked {
    type TimeLocked: TimeLocked<ValueType = Self>;

    fn into_time_locked(self, valid_after: u64) -> Self::TimeLocked;
}

pub trait TimeLocked: IntoVal<Env, Val> + TryFromVal<Env, Val> {
    type ValueType;

    fn valid_after(&self) -> u64;
    fn inner(self) -> Self::ValueType;
}

#[contracttype]
pub struct TimeLockedParameters {
    pub value: Parameters,
    pub valid_after: u64,
}

impl IntoTimeLocked for Parameters {
    type TimeLocked = TimeLockedParameters;

    fn into_time_locked(self, valid_after: u64) -> Self::TimeLocked {
        Self::TimeLocked {
            value: self,
            valid_after,
        }
    }
}

impl TimeLocked for TimeLockedParameters {
    type ValueType = Parameters;

    fn valid_after(&self) -> u64 {
        self.valid_after
    }

    fn inner(self) -> Self::ValueType {
        self.value
    }
}

#[contracttype]
pub struct TimeLockedGovernanceParameters {
    pub value: GovernanceParameters,
    pub valid_after: u64,
}

impl IntoTimeLocked for GovernanceParameters {
    type TimeLocked = TimeLockedGovernanceParameters;

    fn into_time_locked(self, valid_after: u64) -> Self::TimeLocked {
        Self::TimeLocked {
            value: self,
            valid_after,
        }
    }
}

impl TimeLocked for TimeLockedGovernanceParameters {
    type ValueType = GovernanceParameters;

    fn valid_after(&self) -> u64 {
        self.valid_after
    }

    fn inner(self) -> Self::ValueType {
        self.value
    }
}

#[contracttype]
pub struct TimeLockedAddress {
    pub value: Address,
    pub valid_after: u64,
}

impl IntoTimeLocked for Address {
    type TimeLocked = TimeLockedAddress;

    fn into_time_locked(self, valid_after: u64) -> Self::TimeLocked {
        Self::TimeLocked {
            value: self,
            valid_after,
        }
    }
}

impl TimeLocked for TimeLockedAddress {
    type ValueType = Address;

    fn valid_after(&self) -> u64 {
        self.valid_after
    }

    fn inner(self) -> Self::ValueType {
        self.value
    }
}

#[contracttype]
pub struct TimeLockedWasm {
    pub value: BytesN<32>,
    pub valid_after: u64,
}

impl IntoTimeLocked for BytesN<32> {
    type TimeLocked = TimeLockedWasm;

    fn into_time_locked(self, valid_after: u64) -> Self::TimeLocked {
        Self::TimeLocked {
            value: self,
            valid_after,
        }
    }
}

impl TimeLocked for TimeLockedWasm {
    type ValueType = BytesN<32>;

    fn valid_after(&self) -> u64 {
        self.valid_after
    }

    fn inner(self) -> Self::ValueType {
        self.value
    }
}
