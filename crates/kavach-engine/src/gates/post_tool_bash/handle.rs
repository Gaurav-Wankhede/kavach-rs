//! `PostToolUse:Bash` handler: sync events to memory (test/build/deploy/db
//! tracking) and inject actionable context on failure signals.
use kavach_types::HookInput;

use super::detect::{
    detect_port_conflict, is_empty_test_suite, is_package_install, is_package_not_found,
    is_test_command,
};
use super::progress::track_db_progress;
use super::tests_track::{clear_test_run, resolve_tested_files};
use crate::error::EngineError;

/// Handle bash done: sync events to memory (build, test, deploy tracking).
///
/// # Errors
/// Returns `Ok(())` on every path; the `Result` matches the uniform handler
/// dispatch in `post_tool::run` so all tool arms share one return type.
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature is fixed by the post_tool::run match dispatch: every per-tool handler returns Result<(), EngineError> so the arms share one type"
)]
pub(crate) fn handle(
    input: &HookInput,
    session: &mut kavach_session::SessionState,
) -> Result<(), EngineError> {
    let command = input.get_string("command");

    // Capture this Bash command to the session trajectory tape (replay/reward
    // signal). Best-effort: a tape-write error must never block the gate.
    capture_bash(&session.session_id, command);

    let output = input
        .tool_response
        .as_ref()
        .and_then(|r| r.get("output"))
        .and_then(|v| v.as_str());

    if is_test_command(command) {
        clear_test_run(session, command);
        if is_empty_test_suite(output) {
            session.add_case_fact("EMPTY_TEST_SUITE: 0 tests ran — not a pass");
            let context = "[EMPTY_TEST_SUITE] 0 tests ran. This is NOT a passing test suite. \
                 Either: (1) no tests exist for the modified code, or \
                 (2) test filtering excluded all tests. \
                 Do NOT claim \"all tests pass\" — zero tests ran.";
            drop(kavach_hook::exit_post_tool_context(context));
            return Ok(());
        }
        resolve_tested_files(session, command);
    }

    track_db_progress(session, command);

    if is_package_install(command) && is_package_not_found(output) {
        session.add_case_fact("PACKAGE_NOT_FOUND: hallucinated package name");
        let context = "[PACKAGE_NOT_FOUND] Package does not exist on the registry. \
             You hallucinated the package name from training weights. \
             WebSearch the correct package name before retrying. \
             Do NOT guess — research it.";
        drop(kavach_hook::exit_post_tool_context(context));
        return Ok(());
    }

    if let Some(port) = detect_port_conflict(output) {
        session.add_case_fact(&format!("PORT_CONFLICT: port {port} already in use"));
        #[expect(clippy::arithmetic_side_effects, reason = "port is u16; +1 is bounded")]
        let alt_port = port + 1;
        let context = format!(
            "[PORT_CONFLICT] Port {port} is already in use.\n\
             Options:\n\
             1. Use alternative port: --port {alt_port}\n\
             2. Find what's using it: `procs --insert TcpPort {port}` (toolbelt) or `lsof -i:{port}`\n\
             3. Kill existing process: `lsof -ti:{port} | xargs kill -9`\n\
             Do NOT retry with same port — it will fail again."
        );
        drop(kavach_hook::exit_post_tool_context(&context));
        return Ok(());
    }

    drop(kavach_hook::exit_silent());
    Ok(())
}

/// Append a Bash command to the session trajectory tape. Fire-and-forget:
/// errors are swallowed because a tape write must never block the gate.
fn capture_bash(session_id: &str, command: &str) {
    if session_id.is_empty() || command.is_empty() {
        return;
    }
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    drop(kavach_patterns::eval_replay::capture(
        session_id,
        timestamp_ms,
        kavach_patterns::eval_replay::EventKind::Bash {
            command: command.to_owned(),
        },
    ));
}
