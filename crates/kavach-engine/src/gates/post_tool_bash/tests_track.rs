//! Test-run bookkeeping: clear the active-crate registration and resolve
//! pending test files within the test command's scope.
use kavach_session::SessionState;

/// Remove the crate key from `active_test_crates` when a cargo job completes.
pub(super) fn clear_test_run(session: &mut SessionState, command: &str) {
    let Some(key) = extract_cargo_job_key(command) else {
        return;
    };
    session.active_test_crates.retain(|c| c != &key);
    session.save().ok();
}

/// Extract crate key from a long cargo job — mirrors `pre_tool_bash` logic.
/// MUST cover the same subcommands the pre-tool side registers (test/nextest/
/// build/check); a mismatch would register a key on start but never clear it,
/// leaking a stale lock-key that blocks every later run of that crate.
fn extract_cargo_job_key(cmd: &str) -> Option<String> {
    if !cmd.contains("cargo test")
        && !cmd.contains("cargo nextest")
        && !cmd.contains("cargo build")
        && !cmd.contains("cargo check")
    {
        return None;
    }
    if cmd.contains("--workspace") {
        return Some("__workspace__".into());
    }
    for flag in &["-p ", "--package "] {
        if let Some(pos) = cmd.find(flag)
            && let Some(after) = cmd.get(pos.saturating_add(flag.len())..)
            && let Some(name) = after.split_whitespace().next()
        {
            return Some(name.to_owned());
        }
    }
    Some("__workspace__".into())
}

/// Clear only pending files matching the test scope.
/// `cargo test -p pkg-a` must NOT clear pending TypeScript files.
pub(super) fn resolve_tested_files(session: &mut SessionState, command: &str) {
    if !session.has_pending_tests() {
        return;
    }
    let scope = extract_test_scope(command);
    if scope.is_empty() {
        session.clear_test_pending();
        return;
    }
    session
        .test_files_pending
        .retain(|f| !scope.iter().any(|s| f.contains(s)));
    if session.test_files_pending.is_empty() {
        session.test_nudge_count = 0;
    }
    session.save().ok();
}

pub(super) fn extract_test_scope(cmd: &str) -> Vec<String> {
    let mut scopes = Vec::new();
    if let Some(pos) = cmd.find("-p ")
        && let Some(after) = cmd.get(pos.saturating_add(3)..)
        && let Some(pkg) = after.split_whitespace().next()
    {
        scopes.push(pkg.to_owned());
    }
    for part in cmd.split_whitespace() {
        let lower = part.to_lowercase();
        if part.contains('/')
            || std::path::Path::new(&lower)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("ts") || e.eq_ignore_ascii_case("tsx"))
        {
            scopes.push(part.to_owned());
        }
    }
    scopes
}
