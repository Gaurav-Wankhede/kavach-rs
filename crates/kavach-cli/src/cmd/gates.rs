use kavach_hook::HookAction;
use kavach_types::HookInput;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// `kavach gates <name> --hook` — run a gate, reading JSON from stdin.
/// `kavach gates <name> --verify "prompt"` — dry-run a gate with inline prompt.
/// Without flags, prints gate info.
pub(super) fn run(gate_name: &str, hook: bool, verify: Option<String>) -> i32 {
    if let Some(prompt) = verify {
        return run_verify(gate_name, &prompt);
    }

    if gate_name == "reset-test-enforcement" {
        return reset_test_enforcement();
    }

    if !hook {
        return print_gate_info(gate_name);
    }

    // Read hook input from stdin.
    //
    // FIX rca.hook-error-path-fail-open [contract_violation + silent_failure]
    // — a security gate that cannot evaluate must FAIL CLOSED. The old
    // `return 1` made every parse-failure / gate-error a FALSE-SECURITY
    // bypass: per the Claude Code v2.1.143 contract and the confirmed
    // upstream bug anthropics/claude-code#21988, a non-zero-non-2 exit
    // (incl. the default exit 1 of any uncaught error) is logged as a
    // hook error and the tool PROCEEDS — exactly on the anomalous-input
    // case the gate exists for. The robust, schema-independent fail-closed
    // signal is **exit 2 with the reason on stderr**: it blocks regardless
    // of hook event type.
    let input = match kavach_hook::must_read_hook_input() {
        Ok(input) => input,
        Err(HookAction::Error) => {
            let msg = format!(
                "kavach gate '{gate_name}': unreadable hook input — failing CLOSED \
                 (exit 2). Anomalous stdin must not bypass the gate."
            );
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 2;
        }
        Err(HookAction::Done) => return 0,
    };

    match kavach_engine::run_gate(gate_name, &input) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!(
                "kavach gate '{gate_name}' evaluation error: {e} — failing CLOSED \
                 (exit 2). A gate that cannot complete must not let the tool through."
            );
            2
        }
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
