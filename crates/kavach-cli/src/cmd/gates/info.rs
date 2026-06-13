//! `kavach gates <name>` (no `--hook`/`--verify`) — print a gate's purpose.

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn print_gate_info(gate_name: &str) -> i32 {
    let description = match gate_name {
        "pre-write" => {
            "Security chain + content check + research gate (Write/Edit/NotebookEdit)"
        }
        "post-write" => {
            "Antiprod scan + quality + lint + memory sync (Write/Edit/NotebookEdit)"
        }
        "pre-tool" => "Bash blocklist + read validation + subagent budget (all tools)",
        "post-tool" => {
            "Context injection + research tracking + task sync (all tools)"
        }
        "intent" => {
            "Intent classification + skill routing + CEO delegation (UserPromptSubmit)"
        }
        "subagent-start" => "Subagent lifecycle start + budget injection",
        "subagent-stop" => "Subagent lifecycle stop + output tracking",
        "session-start" => {
            "Session initialization: kanban inject, mistake patterns, six-file context"
        }
        "session-end" => "Session end lifecycle hook + memory flush",
        "pre-compact" => "Pre-compact lifecycle hook — preserve state before context trim",
        "stop" => {
            "Stop lifecycle hook: kanban dispatch, behavioral breaker, AUTO_CONTINUE injection"
        }
        "post-tool-failure" => "Post-tool failure recovery and error tracking",
        "permission" => "Permission elevation gate for sensitive operations",
        "permission-request" => {
            "PermissionRequest event handler (hookSpecificOutput format)"
        }
        "notification" => {
            "Notification dispatch + terminal bell on attention events (CC 2.1.141)"
        }
        "message-display" => {
            "MessageDisplay pass-through transform hook (CC 2.1.152)"
        }
        "teammate-idle" => "Teammate idle detection and task reassignment",
        "task-completed" => "Task completion verification and memory sync",
        "six-file-intent" => {
            "Six-file context: classify user intent against app_spec / roadmap scope"
        }
        "pre-implementation" => {
            "Six-file context: block IMPLEMENT until unit spec + dependencies are loaded"
        }
        "post-implementation" => {
            "Six-file context: verify implementation against unit spec before marking done"
        }
        other => {
            let msg = format!("unknown gate: {other}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            let avail = "available: pre-write, post-write, pre-tool, post-tool, intent, \
subagent-start, subagent-stop, session-start, session-end, pre-compact, stop, \
post-tool-failure, permission, permission-request, notification, message-display, \
teammate-idle, task-completed, six-file-intent, pre-implementation, post-implementation";
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
    let usage = format!(
        "usage: echo '{{}}' | kavach gates {gate_name} --hook [--vendor cursor|claude-code|codex]"
    );
    if let Err(io_err) = print_or_exit(&usage) {
        return into_exit_code(io_err);
    }
    0
}
