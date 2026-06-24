// SOURCE: https://docs.rs/linfa-trees/ — feature extraction for decision trees

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched and constructed cross-crate by ML consumers; non_exhaustive => E0004"
)]
pub enum Feature {
    Boolean(bool),
    Numeric(f64),
    Categorical(String),
}

impl Feature {
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_numeric(&self) -> Option<f64> {
        match self {
            Self::Numeric(n) => Some(*n),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_categorical(&self) -> Option<&str> {
        match self {
            Self::Categorical(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureSet {
    features: HashMap<String, Feature>,
}

impl FeatureSet {
    #[must_use]
    pub fn new() -> Self {
        Self {
            features: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, value: Feature) {
        self.features.insert(name.into(), value);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Feature> {
        self.features.get(name)
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.features.contains_key(name)
    }

    #[must_use]
    pub fn with_bool(mut self, name: impl Into<String>, value: bool) -> Self {
        self.insert(name, Feature::Boolean(value));
        self
    }

    #[must_use]
    pub fn with_numeric(mut self, name: impl Into<String>, value: f64) -> Self {
        self.insert(name, Feature::Numeric(value));
        self
    }
}
