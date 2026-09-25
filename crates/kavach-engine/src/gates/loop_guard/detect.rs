use super::history::{normalize_command, parse_entry, truncate};
use super::inspection::{git_diff_has_pending_changes, is_inspection_command};
use kavach_session::SessionState;

const MAX_REPEATS: usize = 3;
pub(super) const WINDOW_TURNS: i32 = 10;
/// Consecutive identical Bash commands that trigger a hard deny (the
/// advisory above fires at 3-in-window; models demonstrably ignore
/// advisories and spam 15+ identical calls). Five in a row with no
/// interleaving command is never legitimate polling — polling varies the
/// command or observes changed output.
const DENY_CONSECUTIVE: usize = 5;
/// Escape hatch, consistent with KAVACH_DONE_GAMING_BYPASS/KAVACH_TDD_BYPASS
/// house style: explicit operator opt-out for scripted harnesses.
const LOOP_BYPASS_ENV: &str = "KAVACH_LOOP_BYPASS";

pub(crate) fn check_bash_loop(session: &SessionState, command: &str) -> Option<String> {
    let normalized = normalize_command(command);
    let window_start = session.turn_count.saturating_sub(WINDOW_TURNS);

    let count = session
        .recent_commands
        .iter()
        .filter(|entry| {
            let (turn, cmd) = parse_entry(entry);
            turn >= window_start && normalize_command(cmd) == normalized
        })
        .count();

    if count < MAX_REPEATS {
        return None;
    }
    if std::env::var("KAVACH_NEW_DETECTORS").as_deref() == Ok("1")
        && is_inspection_command(&normalized)
        && git_diff_has_pending_changes()
    {
        return None;
    }
    Some(format!(
        "[LOOP_LIMIT] Command executed {count}x in last {WINDOW_TURNS} turns: `{}` \
         — repeating it won't produce different results \
         -> diagnose WHY it fails (don't just retry), try a different approach or tool, \
         or use /clear if context is polluted with failed attempts -> then retry.",
        truncate(command, 80)
    ))
}

/// Hard deny for tight identical-command loops: the current command plus the
/// trailing run of identical normalized commands in history must reach
/// `DENY_CONSECUTIVE`. Consecutive-only (not window-count) so legitimate
/// re-reads across turns never trip it; only an uninterrupted repeat does.
/// Reuses the inspection+diff exemption: re-running a read-only inspection
/// after the tree changed is verification, not a loop. Fails closed on
/// bypass absence; `KAVACH_LOOP_BYPASS=1` opts scripted harnesses out.
///
/// FP bound (<1% for P0): fires only on byte-identical (post-normalization)
/// commands with zero interleaving, excluding inspection-after-mutation.
/// Covered by `deny_*` tests below: 4-in-a-row passes, interleaved passes,
/// whitespace variants count (normalization), exempt path passes.
pub(crate) fn check_bash_loop_deny(session: &SessionState, command: &str) -> Option<String> {
    if std::env::var(LOOP_BYPASS_ENV).as_deref() == Ok("1") {
        return None;
    }
    let normalized = normalize_command(command);
    if normalized.is_empty() {
        return None;
    }
    let mut run = 1usize;
    for entry in session.recent_commands.iter().rev() {
        let (_, cmd) = parse_entry(entry);
        if normalize_command(cmd) == normalized {
            run = run.saturating_add(1);
        } else {
            break;
        }
    }
    if run < DENY_CONSECUTIVE {
        return None;
    }
    if std::env::var("KAVACH_NEW_DETECTORS").as_deref() == Ok("1")
        && is_inspection_command(&normalized)
        && git_diff_has_pending_changes()
    {
        return None;
    }
    Some(format!(
        "[LOOP_DENY] STOP — identical command {run}x in a row: `{}` \
         — the harness is looping. Further repeats are refused. NEXT ACTION (pick one): \
         (1) read the output you already have above and act on it; \
         (2) run the equivalent via a different tool (sg for AST shape, Read for full content); \
         (3) vary the command (narrow the pattern, add context lines); \
         (4) /clear if context is polluted, then retry once. \
         Bypass (scripted harnesses only): {LOOP_BYPASS_ENV}=1.",
        truncate(command, 80)
    ))
}

const TOOL_HISTORY_SIZE: usize = 30;

pub(crate) fn check_tool_loop(
    session: &SessionState,
    tool_name: &str,
    tool_input: &str,
) -> Option<String> {
    let normalized_input = normalize_command(tool_input);
    // SOURCE: https://rust.code-maven.com/automatically-fixing-issues-in-rust
    let key = format!("{tool_name}:{normalized_input}");
    let window_start = session.turn_count.saturating_sub(WINDOW_TURNS);

    let count = session
        .recent_tool_calls
        .iter()
        .filter(|entry| {
            let (turn, recorded) = parse_entry(entry);
            turn >= window_start && recorded == key
        })
        .count();

    if count < MAX_REPEATS {
        return None;
    }
    Some(format!(
        "[LOOP_LIMIT] STOP: Same tool call executed {count}x in last {WINDOW_TURNS} turns: `{tool_name}` \
         — repeating it won't produce different results. \
         DIAGNOSE ROOT CAUSE BEFORE RETRYING: \
         (1) Is the argument correct? \
         (2) Is the file path right? \
         (3) Read the error message carefully. \
         (4) Try a fundamentally different approach or tool. \
         (5) Use /clear if context is polluted with failed attempts."
    ))
}

pub(crate) fn record_tool_call(session: &mut SessionState, tool_name: &str, tool_input: &str) {
    let normalized_input = normalize_command(tool_input);
    let entry = format!("{}:{}:{}", session.turn_count, tool_name, normalized_input);
    session.recent_tool_calls.push(entry);
    if session.recent_tool_calls.len() > TOOL_HISTORY_SIZE {
        session.recent_tool_calls.rotate_left(1);
        session.recent_tool_calls.pop();
    }
}
