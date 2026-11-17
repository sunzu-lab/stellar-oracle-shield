use soroban_sdk::contracttype;

/// oracle shield previous contract parameters
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parameters {
    /// staleness configuration access key
    pub max_staleness: u64,
    /// maximum deviation in points from the median for a fresh score to be considered agreeing
    pub max_deviation: u32,
    /// minimum number of fresh independant scores agreeing with the median score
    /// quorum has to be lower or equal to the number of operators
    pub quorum: u32,
}

impl Parameters {
    const DEFAULT_MAX_STALENESS_SECONDS: u64 = 3600;
    const DEFAULT_MAX_DEVIATION_POINTS: u32 = 5;
    const DEFAULT_QUORUM: u32 = 0;
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            max_staleness: Self::DEFAULT_MAX_STALENESS_SECONDS,
            max_deviation: Self::DEFAULT_MAX_DEVIATION_POINTS,
            quorum: Self::DEFAULT_QUORUM,
        }
    }
}
