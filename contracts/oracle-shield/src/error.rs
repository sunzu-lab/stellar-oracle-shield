use soroban_sdk::contracterror;

/// oracle shield contract errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// admin is not set - unlikely
    MissingAdmin = 701,
    /// score set is out of bounds
    ScoreBounds = 702,
    /// pair is not covered
    PairNotCovered = 703,
    /// input feed is stale
    StaleInput = 704,
    /// conversion error - unlikely
    ConversionError = 705,
    /// max staleness not set (shouldn't happen)
    NoMaxStalenessSet = 706,
}
