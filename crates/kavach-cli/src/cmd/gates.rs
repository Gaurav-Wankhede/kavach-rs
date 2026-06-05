use kavach_hook::Vendor;
use kavach_types::{HookInput, HookResponse};

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

#[cfg(test)]
#[path = "gates_test.rs"]
mod tests;

/// `kavach gates <name> --hook [--vendor v]` — run a gate, reading JSON from
/// stdin in the resolved harness's NATIVE dialect (Claude Code / Cursor / Codex).
/// `kavach gates <name> --verify "prompt"` — dry-run a gate with inline prompt.
/// Without flags, prints gate info.
pub(super) fn run(gate_name: &str, hook: bool, verify: Option<String>, vendor: Option<&str>) -> i32 {
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
        // Unreadable/unparseable input: apply the resolved vendor's NATIVE
        // failure policy. Cursor fails OPEN (allow — never wedge the IDE);
        // Codex/Claude Code fail CLOSED (exit 2 / deny). The decode error itself
        // carries the resolved vendor so we honor the right contract.
        Err((vendor, msg)) => return fail_native(vendor, gate_name, &msg),
    };

    // The engine is vendor-blind: it sees only the canonical input.
    //
    // CONTRACT (gate_runner.rs): every gate handler WRITES ITS OWN native stdout
    // via kavach_hook helpers and returns `Ok(())` — including self-emitted blocks
    // (pre-write deny) AND context injection (session-start's mistake ledger).
    // `Err` means the gate could NOT run (unknown name / handler failure), so it
    // emitted nothing.
    //
    // Therefore the dispatcher must stay SILENT on `Ok` — emitting a second JSON
    // object made Claude Code drop the gate's rich first object, the load-time
    // context-loss bug (mistakes/instructions not loading). It only owns output
    // on `Err`, where it must fail-closed in the harness's native contract.
    match kavach_engine::run_gate(gate_name, &input) {
        Ok(()) => 0,
        Err(e) => fail_native(resolved, gate_name, &e.to_string()),
    }
}

/// Emit a vendor-native FAILURE for an unreadable payload and return its exit code.
///
/// Cursor's native model is fail-OPEN, so a decode failure must let the action
/// through (a blocked-on-error gate wedges the editor — the original Cursor bug).
/// Codex and Claude Code fail CLOSED with the reason on stderr.
fn fail_native(vendor: Vendor, gate_name: &str, msg: &str) -> i32 {
    let resp = HookResponse::new_block(&format!(
        "kavach gate '{gate_name}': unreadable hook input ({msg})"
    ));
    // Cursor's native model: render its fail-OPEN body and exit 0 so the action
    // proceeds — a blocked-on-error gate wedges the editor.
    if vendor == Vendor::Cursor {
        eprintln!("kavach gate '{gate_name}': unreadable input — Cursor fail-OPEN (allow)");
        return kavach_hook::output_native(vendor, &HookResponse::new_approve("fail-open"));
    }
    // Codex / Claude Code: fail CLOSED on anomalous stdin. Emit the native block
    // body, then force **exit 2** — the schema-independent block signal both
    // honor regardless of event type (the original gates.rs contract;
    // anthropics/claude-code#21988). `.max(2)` keeps Codex's native 2 and lifts
    // Claude Code's body-block (exit 0, which an error-logged unreadable payload
    // would otherwise treat as a bypass) to a real block.
    eprintln!(
        "kavach gate '{gate_name}': unreadable input ({msg}) — failing CLOSED (exit 2). \
         Anomalous stdin must not bypass the gate."
    );
    kavach_hook::output_native(vendor, &resp).max(2)
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

fn print_gate_info(gate_name: &str) -> i32 {
    let description = match gate_name {
        "pre-write" => "Security chain + content check + research gate (Write/Edit/NotebookEdit)",
        "post-write" => "Antiprod scan + quality + lint + memory sync (Write/Edit/NotebookEdit)",
        "pre-tool" => "Bash blocklist + read validation + subagent budget (all tools)",
        "post-tool" => "Context injection + research tracking + task sync (all tools)",
        "intent" => "Intent classification + skill routing + CEO delegation (UserPromptSubmit)",
        "subagent-start" => "Subagent lifecycle start + budget injection",
        "subagent-stop" => "Subagent lifecycle stop + output tracking",
        "session-start" => "Session initialization lifecycle hook",
        "session-end" => "Session end lifecycle hook",
        "pre-compact" => "Pre-compact lifecycle hook",
        "stop" => "Stop lifecycle hook",
        "post-tool-failure" => "Post-tool failure recovery and error tracking",
        "permission" => "Permission elevation gate for sensitive operations",
        "permission-request" => "PermissionRequest event handler (hookSpecificOutput format)",
        "notification" => "Notification dispatch + terminal bell on attention events (CC 2.1.141)",
        "message-display" => "MessageDisplay pass-through transform hook (CC 2.1.152)",
        "teammate-idle" => "Teammate idle detection and task reassignment",
        "task-completed" => "Task completion verification and memory sync",
        other => {
            let msg = format!("unknown gate: {other}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            let avail = "available: pre-write, post-write, pre-tool, post-tool, intent, subagent-start, subagent-stop, session-start, session-end, pre-compact, stop, post-tool-failure, permission, permission-request, notification, teammate-idle, task-completed";
            if let Err(io_err) = ewrite_or_exit(avail) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let info_line = format!("{gate_name}: {description}");
    if let Err(io_err) = print_or_exit(&info_line) {
        return into_exit_code(io_err);
    }
    let usage = format!("usage: echo '{{}}' | kavach gates {gate_name} --hook");
    if let Err(io_err) = print_or_exit(&usage) {
        return into_exit_code(io_err);
    }
    0
}
