use {
    crate::{error::Error, status::Status},
    soroban_sdk::{Env, IntoVal, TryFromVal, Val},
};

/// u32 with range check
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    pub score: u32,
    pub ts: u64,
}

impl Score {
    const MAX_SCORE: u32 = 100;

    pub fn new(score: u32, ts: u64) -> Result<Self, Error> {
        if score <= Self::MAX_SCORE {
            Ok(Self { score, ts })
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
        let (s, ts) = <(u32, u64)>::try_from_val(env, v).map_err(|_| Error::ConversionError)?;
        Self::new(s, ts)
    }
}

impl IntoVal<Env, Val> for Score {
    fn into_val(&self, e: &Env) -> Val {
        (self.score, self.ts).into_val(e)
    }
}
