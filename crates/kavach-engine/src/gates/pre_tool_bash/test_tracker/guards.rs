//! Cargo-job gates: unscoped-workspace block (test only), duplicate-run block
//! (test/build/check), registration. One tracker covers every long cargo job
//! that contends on the shared `target/` lock.
use super::extract::extract_cargo_job_key;

/// HARD BLOCK: unscoped `cargo test`/`cargo nextest run` without `-p <crate>`.
/// Full workspace tests take 10-20 min. Force `-p <crate>` for the modified crate.
/// Allows explicit `--workspace` flag (intentional full run). Scoped to TESTS:
/// an unscoped `cargo build`/`check` is the normal whole-workspace verify and is
/// NOT blocked here — only the *duplicate* of any cargo job is (see below).
pub(in crate::gates::pre_tool_bash) fn check_unscoped_test_run(cmd: &str) -> Option<String> {
    if !(cmd.contains("test") || cmd.contains("nextest")) {
        return None;
    }
    let key = extract_cargo_job_key(cmd)?;
    if key != "__workspace__" {
        return None;
    }
    if cmd.contains("--workspace") {
        return None;
    }
    Some(
        "UNSCOPED_TEST_BLOCKED: `cargo test`/`cargo nextest run` without `-p <crate>` \
         runs the FULL workspace (10-20 min), blocking all other operations.\n\
         FIX: Scope to the crate you modified:\n\
         cargo nextest run -p <crate-name>\n\
         Or use -E filterset: cargo nextest run -E 'test(my_test)'\n\
         Only use --workspace for final pre-merge verification."
            .to_owned(),
    )
}

/// Block if this crate already has a long cargo job (`test`/`nextest`/`build`/
/// `check`) running — the duplicate would only block on the shared `target/`
/// lock and waste a shell (the observed "two shells for the same build" waste).
/// Auto-expires stale entries: clears `active_test_crates` if 5+ turns have
/// passed since `last_write_turn` (the job likely finished or was interrupted).
pub(in crate::gates::pre_tool_bash) fn check_duplicate_test_run(
    session: &kavach_session::SessionState,
    cmd: &str,
) -> Option<String> {
    let key = extract_cargo_job_key(cmd)?;
    // Auto-expire: if 5+ turns have passed since last_write_turn, the tracked
    // job likely finished — skip the block (register_test_run overwrites on the
    // next real job start).
    if !session.active_test_crates.is_empty() {
        let stale_threshold = session.turn_count.saturating_sub(5);
        if session.last_write_turn < stale_threshold {
            return None;
        }
    }
    if !session.active_test_crates.iter().any(|c| c == &key) {
        return None;
    }
    let display = if key == "__workspace__" {
        "the workspace".to_owned()
    } else {
        format!("crate `{key}`")
    };
    Some(format!(
        "CARGO_JOB_ALREADY_RUNNING: a cargo build/check/test for {display} is already \
         in progress.\n\
         Do NOT launch a second shell for the same job — wait for the running one.\n\
         Cargo allows ONE process to hold the `target/` lock at a time, so the \
         duplicate just blocks on 'Blocking waiting for file lock' and wastes a shell \
         (Cargo's single-lock design is intentional).\n\
         FIX: Check the background tasks panel and read the running job's exit code \
         instead of re-running it.\n\
         SOURCE: https://doc.rust-lang.org/cargo/guide/build-cache.html"
    ))
}

/// Register a cargo-job start — adds crate key to `active_test_crates`.
pub(in crate::gates::pre_tool_bash) fn register_test_run(
    session: &mut kavach_session::SessionState,
    cmd: &str,
) {
    if let Some(key) = extract_cargo_job_key(cmd)
        && !session.active_test_crates.iter().any(|c| c == &key)
    {
        session.active_test_crates.push(key);
        session.save().ok();
    }
}
