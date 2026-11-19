use soroban_sdk::contracterror;

/// oracle shield contract errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[non_exhaustive]
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
    /// operator is not registered
    UnauthorizedOperator = 707,
    /// quorum not reached
    QuorumNotReached = 708,
    /// time-locked operation - too early
    TimeLocked = 709,
    /// expired operation - too late
    Expired = 710,
    /// a scheduled operation is already in progress - cancel first
    ScheduledOperationInProgress = 711,
    /// invalid parameter
    InvalidParam = 712,
}
