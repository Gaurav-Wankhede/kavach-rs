//! Rule engine for evaluating TOON skill rules during hook execution.
//! Bridges kavach-rule-ast + kavach-rule-parser with the gate system.

pub mod compliance;
pub mod context;
pub mod enforce;
pub mod engine;
pub mod file_matcher;
pub mod matcher;
pub mod research;
pub mod result;

pub use context::EvalContext;
pub use engine::RuleEngine;
pub use result::{RuleAction, RuleResult};
