//! Artifact-shape validators: maps each `ArtifactValidator` to a keyword or
//! compound-predicate check used by the six-file witness gate.
mod helpers;
mod shapes;
mod validate;
#[cfg(test)]
#[path = "validators_test.rs"]
mod tests;
pub(crate) use validate::validate;
