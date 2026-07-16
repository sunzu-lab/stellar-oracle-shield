use {crate::score::Score, soroban_sdk::contracttype};

/// pairs status
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// Protocol may proceed.
    Healthy,
    /// Protocol may proceed under stricter risk parameters (for example wider slippage, lower LTV, smaller size).
    Degraded,
    /// Protocol should halt oracle-dependent operations.
    Unsafe,
}

impl From<Score> for Status {
    fn from(score: Score) -> Status {
        const HEALTHY_THRESHOLD: u32 = 66;
        const DEGRADED_THRESHOLD: u32 = 33;
        let score = score.score;
        if score >= HEALTHY_THRESHOLD {
            Self::Healthy
        } else if score >= DEGRADED_THRESHOLD {
            Self::Degraded
        } else {
            Self::Unsafe
        }
    }
}
