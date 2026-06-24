// SOURCE: https://github.com/Entscheider/stamm — tree traversal and serialization

use crate::error::DTreeError;
use crate::feature::FeatureSet;
use crate::node::{DecisionNode, Outcome};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTree {
    root: DecisionNode,
    name: String,
    version: u32,
}

impl DecisionTree {
    pub fn new(name: impl Into<String>, root: DecisionNode) -> Self {
        Self {
            root,
            name: name.into(),
            version: 1,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Classifies the given feature set by traversing the decision tree.
    ///
    /// # Errors
    /// Returns `DTreeError` if evaluation fails due to missing or invalid features.
    pub fn classify(&self, features: &FeatureSet) -> Result<&Outcome, DTreeError> {
        self.root.evaluate(features)
    }

}
