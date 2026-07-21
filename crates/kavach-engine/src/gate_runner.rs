//! Gate dispatch entry point. `run_gate` matches a gate name against the two
//! gate families (core write/tool, lifecycle) in turn, returning
//! `UnknownGate` only when no family claims it.
mod core_gates;
mod lifecycle_gates;
mod util;
#[cfg(test)]
#[path = "gate_runner_test.rs"]
mod tests;
use kavach_types::HookInput;
use crate::error::EngineError;
/// Dispatch a gate by name. Called by the CLI with parsed `HookInput`.
/// Each gate handler reads session state, runs checks, and writes
/// JSON response to stdout via `kavach_hook` helpers.
///
/// # Errors
/// Returns `EngineError` if the gate name is unknown or a gate handler fails.
pub fn run_gate(gate_name: &str, input: &HookInput) -> Result<(), EngineError> {
    core_gates::dispatch(gate_name, input)
        .or_else(|| lifecycle_gates::dispatch(gate_name, input))
        .unwrap_or_else(|| Err(EngineError::UnknownGate(gate_name.to_owned())))
}
