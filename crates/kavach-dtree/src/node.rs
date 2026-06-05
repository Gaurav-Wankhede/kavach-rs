// SOURCE: https://github.com/Entscheider/stamm — generic decision tree node structure
// SOURCE: https://docs.rs/linfa-trees/ — split value predicate pattern

use crate::error::DTreeError;
use crate::feature::FeatureSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Predicate {
    FeatureEquals { name: String, value: String },
    FeatureContains { name: String, substring: String },
    NumericLessThan { name: String, threshold: f64 },
    NumericGreaterThan { name: String, threshold: f64 },
    BooleanIs { name: String, expected: bool },
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Not(Box<Self>),
}

impl Predicate {
    /// Evaluate this predicate against a feature set.
    ///
    /// # Errors
    ///
    /// Returns `DTreeError::FeatureMissing` if a required feature is not found in the feature set.
    pub fn evaluate(&self, features: &FeatureSet) -> Result<bool, DTreeError> {
        match self {
            Self::FeatureEquals { name, value } => {
                let feat = features
                    .get(name)
                    .ok_or_else(|| DTreeError::FeatureMissing(name.clone()))?;
                Ok(feat.as_categorical().is_some_and(|v| v == value))
            }
            Self::FeatureContains { name, substring } => {
                let feat = features
                    .get(name)
                    .ok_or_else(|| DTreeError::FeatureMissing(name.clone()))?;
                Ok(feat
                    .as_categorical()
                    .is_some_and(|v| v.contains(substring.as_str())))
            }
            Self::NumericLessThan { name, threshold } => {
                let feat = features
                    .get(name)
                    .ok_or_else(|| DTreeError::FeatureMissing(name.clone()))?;
                Ok(feat.as_numeric().is_some_and(|n| n < *threshold))
            }
            Self::NumericGreaterThan { name, threshold } => {
                let feat = features
                    .get(name)
                    .ok_or_else(|| DTreeError::FeatureMissing(name.clone()))?;
                Ok(feat.as_numeric().is_some_and(|n| n > *threshold))
            }
            Self::BooleanIs { name, expected } => {
                let feat = features
                    .get(name)
                    .ok_or_else(|| DTreeError::FeatureMissing(name.clone()))?;
                Ok(feat.as_bool().is_some_and(|b| b == *expected))
            }
            Self::And(left, right) => Ok(left.evaluate(features)? && right.evaluate(features)?),
            Self::Or(left, right) => Ok(left.evaluate(features)? || right.evaluate(features)?),
            Self::Not(inner) => Ok(!inner.evaluate(features)?),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate (kavach-chain intent trees, kavach-rpc trust); non_exhaustive => E0639"
)]
pub struct Outcome {
    pub intent_type: String,
    pub complexity: String,
    pub risk_level: String,
    pub required_skills: Vec<String>,
    pub requires_research: bool,
    pub confidence: f64,
}

impl Default for Outcome {
    fn default() -> Self {
        Self {
            intent_type: "general".into(),
            complexity: "simple".into(),
            risk_level: "low".into(),
            required_skills: Vec::new(),
            requires_research: true,
            confidence: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecisionNode {
    Leaf(Outcome),
    Branch {
        predicate: Predicate,
        if_true: Box<Self>,
        if_false: Box<Self>,
    },
}

impl DecisionNode {
    #[must_use]
    pub const fn leaf(outcome: Outcome) -> Self {
        Self::Leaf(outcome)
    }

    #[must_use]
    pub fn branch(predicate: Predicate, if_true: Self, if_false: Self) -> Self {
        Self::Branch {
            predicate,
            if_true: Box::new(if_true),
            if_false: Box::new(if_false),
        }
    }

    /// Evaluate this decision tree against a feature set to reach an outcome.
    ///
    /// # Errors
    ///
    /// Returns `DTreeError::FeatureMissing` if a required feature is not found in the feature set.
    pub fn evaluate(&self, features: &FeatureSet) -> Result<&Outcome, DTreeError> {
        match self {
            Self::Leaf(outcome) => Ok(outcome),
            Self::Branch {
                predicate,
                if_true,
                if_false,
            } => {
                if predicate.evaluate(features)? {
                    if_true.evaluate(features)
                } else {
                    if_false.evaluate(features)
                }
            }
        }
    }
}
