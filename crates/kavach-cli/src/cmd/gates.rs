use kavach_types::HookInput;

use crate::cmd::io_safe::{into_exit_code, print_or_exit};

mod fail;
mod info;

use fail::{fail_gate_error, fail_unreadable};
use info::print_gate_info;

/// `kavach gates <name> --hook [--vendor v]` — run a gate, reading JSON from
/// stdin in the resolved harness's NATIVE dialect (Claude Code / Cursor / Codex).
/// `kavach gates <name> --verify "prompt"` — dry-run a gate with inline prompt.
/// Without flags, prints gate info.
pub(super) fn run(
    gate_name: &str,
    hook: bool,
    verify: Option<String>,
    vendor: Option<&str>,
) -> i32 {
    if let Some(prompt) = verify {
        return run_verify(gate_name, &prompt);
    }

    if gate_name == "reset-test-enforcement" {
        return reset_test_enforcement();
    }

    if !hook {
        return print_gate_info(gate_name);
    }

    // Read stdin through the NATIVE EDGE: resolve the harness (hybrid — --vendor
    // flag > $KAVACH_HARNESS > payload sniff > Claude Code) and lower its native
    // payload into the canonical HookInput the engine reasons over.
    let (resolved, input) = match kavach_hook::read_hook_input_native(vendor) {
        Ok(pair) => pair,
        // Unreadable/unparseable input — the gate never ran. Apply the resolved
        // vendor's NATIVE unreadable-input policy (Cursor fails OPEN so a parse
        // glitch never wedges the IDE; Codex/Claude Code fail CLOSED exit 2).
        Err((vendor, msg)) => return fail_unreadable(vendor, gate_name, &msg),
    };

    // The engine is vendor-blind: it sees only the canonical input.
    //
    // CONTRACT (gate_runner.rs): every gate handler WRITES ITS OWN native stdout
    // via kavach_hook helpers and returns `Ok(())` — including self-emitted blocks
    // (pre-write deny) AND context injection (session-start's mistake ledger).
    // `Err` means the gate could NOT run (unknown name / handler failure / RPC-DB
    // outage), so it emitted nothing.
    //
    // Therefore the dispatcher must stay SILENT on `Ok` — emitting a second JSON
    // object made Claude Code drop the gate's rich first object, the load-time
    // context-loss bug. It only owns output on `Err`.
    match kavach_engine::run_gate(gate_name, &input) {
        Ok(()) => 0,
        // The gate RAN but errored — RPC/DB outage. Enforcement gates fail CLOSED
        // here on EVERY harness (Cursor included); observational gates fail OPEN.
        Err(e) => fail_gate_error(resolved, gate_name, &e.to_string()),
    }
}

/// Clear stuck test enforcement state from the current session.
fn reset_test_enforcement() -> i32 {
    let mut session = kavach_session::get_or_create_session();
    let count = session.test_files_pending.len();
    session.clear_test_pending();
    let msg = format!("reset: cleared {count} pending test file(s), nudge count reset to 0");
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

/// Dry-run a gate with an inline prompt string (no stdin JSON needed).
fn run_verify(gate_name: &str, prompt: &str) -> i32 {
    let input = HookInput {
        hook_event_name: "UserPromptSubmit".to_owned(),
        prompt: prompt.to_owned(),
        ..HookInput::default()
    };

    match kavach_engine::run_gate(gate_name, &input) {
        Ok(()) => 0,
        Err(e) => {
            kavach_hook::output_error(&e.to_string());
            1
        }
    }
}
