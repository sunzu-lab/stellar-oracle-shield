use {
    crate::{error::Error, status::Status},
    soroban_sdk::{Env, IntoVal, TryFromVal, Val},
};

/// i32 with range check
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(i32);

impl Score {
    const MIN_SCORE: i32 = 0;
    const MAX_SCORE: i32 = 100;

    pub fn new(value: i32) -> Result<Self, Error> {
        if (Self::MIN_SCORE..=Self::MAX_SCORE).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::ScoreBounds)
        }
    }

    pub fn get(self) -> i32 {
        self.0
    }

    pub fn status(self) -> Status {
        self.into()
    }
}

impl TryFromVal<Env, Val> for Score {
    type Error = Error;

    fn try_from_val(env: &Env, v: &Val) -> Result<Self, Self::Error> {
        let i = i32::try_from_val(env, v).map_err(|_| Error::ConversionError)?;
        Self::new(i)
    }
}

impl IntoVal<Env, Val> for Score {
    fn into_val(&self, e: &Env) -> Val {
        self.get().into_val(e)
    }
}
