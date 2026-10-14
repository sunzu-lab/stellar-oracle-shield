use {
    soroban_sdk::{Env, IntoVal, TryFromVal, TryIntoVal, Val},
    stellar_oracle_shield_client::{Error, Status},
};

/* --- Score --- */

/// u32 with range check
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    pub score: u32,
}

impl Score {
    const MAX_SCORE: u32 = 100;

    pub fn new(score: u32) -> Result<Self, Error> {
        if score <= Self::MAX_SCORE {
            Ok(Self { score })
        } else {
            Err(Error::ScoreBounds)
        }
    }

    pub fn status(self) -> Status {
        self.into()
    }
}

impl TryFromVal<Env, Val> for Score {
    type Error = Error;

    fn try_from_val(env: &Env, v: &Val) -> Result<Self, Self::Error> {
        let i = u32::try_from_val(env, v).map_err(|_| Error::ConversionError)?;
        Self::new(i)
    }
}

impl IntoVal<Env, Val> for Score {
    fn into_val(&self, e: &Env) -> Val {
        self.score.into_val(e)
    }
}

impl TryIntoVal<Env, Val> for Score {
    type Error = Error;

    fn try_into_val(&self, e: &Env) -> Result<Val, Self::Error> {
        Ok(self.into_val(e))
    }
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

/* --- TimestampedScore --- */

/// u32 with range check
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimestampedScore {
    pub score: Score,
    pub ts: u64,
}

impl TimestampedScore {
    pub fn new(score: u32, ts: u64) -> Result<Self, Error> {
        let score = Score::new(score)?;
        Ok(Self { score, ts })
    }
}

impl TryFromVal<Env, Val> for TimestampedScore {
    type Error = Error;

    fn try_from_val(env: &Env, v: &Val) -> Result<Self, Self::Error> {
        let (score, ts) =
            <(Score, u64)>::try_from_val(env, v).map_err(|_| Error::ConversionError)?;
        Ok(Self { score, ts })
    }
}

impl IntoVal<Env, Val> for TimestampedScore {
    fn into_val(&self, e: &Env) -> Val {
        (self.score, self.ts).into_val(e)
    }
}

/* --- AggregatedScore --- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregatedScore {
    pub score: Score,
    pub oldest_ts: u64,
    pub ts: u64,
    pub consensus: u32,
    pub num_operators: u32,
}

impl AggregatedScore {
    #[cfg(test)]
    pub fn new(
        score: u32,
        oldest_ts: u64,
        ts: u64,
        consensus: u32,
        num_operators: u32,
    ) -> Result<Self, Error> {
        let score = Score::new(score)?;
        Ok(Self {
            score,
            oldest_ts,
            ts,
            consensus,
            num_operators,
        })
    }

    pub fn status(self) -> Status {
        self.score.status()
    }
}
