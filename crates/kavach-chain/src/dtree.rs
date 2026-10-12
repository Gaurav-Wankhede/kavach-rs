// SOURCE: https://docs.rs/linfa-trees/ — linfa decision tree structure
// SOURCE: https://github.com/Entscheider/stamm — generic decision trees for Rust

mod error;
mod feature;
mod node;
mod tree;

pub use error::DTreeError;
pub use feature::{Feature, FeatureSet};
pub use node::{DecisionNode, Outcome, Predicate};
pub use tree::DecisionTree;
