use soroban_sdk::contracttype;

const DEFAULT_MAX_STALENESS_SECONDS: u64 = 3600;
const DEFAULT_MAX_DEVIATION_POINTS: u32 = 5;
const DEFAULT_QUORUM: u32 = 1;

/// oracle shield contract parameters
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parameters {
    /// staleness configuration access key
    pub max_staleness: u64,
    /// maximum deviation in points from the median for a fresh score to be considered agreeing
    pub max_deviation: u32,
    /// minimum number of fresh scores agreeing with the median score
    pub quorum: u32,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            max_staleness: DEFAULT_MAX_STALENESS_SECONDS,
            max_deviation: DEFAULT_MAX_DEVIATION_POINTS,
            quorum: DEFAULT_QUORUM,
        }
    }
}
