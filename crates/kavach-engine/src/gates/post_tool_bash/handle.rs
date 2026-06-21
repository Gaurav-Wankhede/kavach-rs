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

    let output = input
        .tool_response
        .as_ref()
        .and_then(|r| r.get("output"))
        .and_then(|v| v.as_str());

    // Capture this Bash command to the session trajectory tape (replay/reward
    // signal) WITH its objective outcome — the ground-truth oracle reads this,
    // not the agent's prose. Best-effort: a tape-write error must never block.
    capture_bash(&session.session_id, command, objective_outcome(input, command, output));

    if is_test_command(command) {
        clear_test_run(session, command);
        // Red-phase capture: a test run that FAILED proves the tests touched this
        // turn are genuinely Red (they fail without the production code). The TDD
        // gate requires this — a mere test-file touch is not test-first.
        if matches!(
            objective_outcome(input, command, output),
            Some(kavach_patterns::eval_replay::EventOutcome::Failure)
        ) {
            tests_track::record_red_units(session);
        }
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

/// Derive the objective `EventOutcome` of a Bash command from artifacts the agent
/// cannot fake — the tool-response error flag and the failure markers a build/test
/// runner prints to its own output. Returns `None` for a command with no
/// gradeable outcome (the oracle treats `None` as silence, never contradiction),
/// so this only ever asserts Success/Failure when the artifact is unambiguous.
fn objective_outcome(
    input: &HookInput,
    command: &str,
    output: Option<&str>,
) -> Option<kavach_patterns::eval_replay::EventOutcome> {
    use kavach_patterns::eval_replay::EventOutcome;
    // Only verification commands (build/test) carry a meaningful pass/fail; a bare
    // `ls`/`cd` is not gradeable, so it stays silent (None).
    if !kavach_patterns::reward::is_real_verify(command) {
        return None;
    }
    // Hard signal 1: the host marked the tool call an error / interrupted it.
    let resp = input.tool_response.as_ref();
    let host_error = resp
        .and_then(|r| r.get("is_error").or_else(|| r.get("interrupted")))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if host_error {
        return Some(EventOutcome::Failure);
    }
    // Hard signal 2: the runner printed a failure marker to its own output. These
    // are emitted by the tool, not narrated by the agent.
    if let Some(out) = output {
        const FAILURE_MARKERS: &[&str] = &[
            "error[E", "error: could not compile", "test result: FAILED", "FAILED",
            "panicked at", "Compilation failed", "Build failed", "error: test failed",
        ];
        if FAILURE_MARKERS.iter().any(|m| out.contains(m)) {
            return Some(EventOutcome::Failure);
        }
    }
    // A real verify command that neither the host nor the runner flagged as failed
    // is an objective Success.
    Some(EventOutcome::Success)
}

/// Append a Bash command + its objective outcome to the session trajectory tape.
/// Fire-and-forget: errors are swallowed because a tape write must never block.
fn capture_bash(
    session_id: &str,
    command: &str,
    outcome: Option<kavach_patterns::eval_replay::EventOutcome>,
) {
    if session_id.is_empty() || command.is_empty() {
        return;
    }
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    drop(kavach_patterns::eval_replay::capture_with_outcome(
        session_id,
        timestamp_ms,
        kavach_patterns::eval_replay::EventKind::Bash {
            command: command.to_owned(),
        },
        outcome,
    ));
}
