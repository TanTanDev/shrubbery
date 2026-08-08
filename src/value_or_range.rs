//! data types for handling user defiend constant or randomized value
use rand::RngExt;
use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueOrRangeU32 {
    Value(u32),
    /// Inclusive `[min, max]`.
    Range(u32, u32),
}

impl Default for ValueOrRangeU32 {
    fn default() -> Self {
        Self::Value(1)
    }
}

impl ValueOrRangeU32 {
    pub fn get(&self, rng: &mut ChaCha8Rng) -> u32 {
        match self {
            ValueOrRangeU32::Value(v) => *v,
            ValueOrRangeU32::Range(min, max) => rng.random_range((*min).min(*max)..=*max),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueOrRangeF32 {
    Value(f32),
    /// Inclusive `[min, max]`.
    Range(f32, f32),
}

impl Default for ValueOrRangeF32 {
    fn default() -> Self {
        Self::Value(1.0)
    }
}

impl ValueOrRangeF32 {
    pub fn max(&self) -> f32 {
        match self {
            ValueOrRangeF32::Value(v) => *v,
            ValueOrRangeF32::Range(_, m) => *m,
        }
    }
    pub fn get(&self, rng: &mut ChaCha8Rng) -> f32 {
        match self {
            ValueOrRangeF32::Value(v) => *v,
            ValueOrRangeF32::Range(min, max) => rng.random_range((*min).min(*max)..=*max),
        }
    }
}
