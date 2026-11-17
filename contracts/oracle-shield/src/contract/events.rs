use {
    super::time_locked::*,
    soroban_sdk::{Address, BytesN, contractevent},
    stellar_oracle_shield_client::{GovernanceParameters, Parameters, Status},
};

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

#[contractevent(data_format = "single-value")]
pub struct ParametersChangeInitiated {
    pub parameters: TimeLockedParameters,
}

#[contractevent(data_format = "single-value")]
pub struct ParametersChangeCancelled {
    pub parameters: Parameters,
}

#[contractevent(data_format = "single-value")]
pub struct ParametersChange {
    pub parameters: Parameters,
}

#[contractevent(data_format = "single-value")]
pub struct GovernanceParamsChangeInit {
    pub parameters: TimeLockedGovernanceParameters,
}

#[contractevent(data_format = "single-value")]
pub struct GovernanceParamsChangeCancel {
    pub parameters: GovernanceParameters,
}

#[contractevent(data_format = "single-value")]
pub struct GovernanceParamsChange {
    pub parameters: GovernanceParameters,
}

#[contractevent(data_format = "single-value")]
pub struct UpgradeInitiated {
    pub wasm_hash: TimeLockedWasm,
}

#[contractevent(data_format = "single-value")]
pub struct UpgradeCancelled {
    pub wasm_hash: BytesN<32>,
}

#[contractevent(data_format = "single-value")]
pub struct UpgradeExecuted {
    pub wasm_hash: BytesN<32>,
}

#[contractevent(data_format = "single-value")]
pub struct AdminTransferInitiated {
    pub admin: TimeLockedAddress,
}

#[contractevent(data_format = "single-value")]
pub struct AdminTransferCancelled {
    pub admin: Address,
}

#[contractevent(data_format = "single-value")]
pub struct AdminTransfered {
    pub admin: Address,
}
