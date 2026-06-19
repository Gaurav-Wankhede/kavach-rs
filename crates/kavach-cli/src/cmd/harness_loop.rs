// ARCH: AutonomousLoopHandler — harness engineering loop-until-complete
// PATTERN: pev_loop (Plan-Execute-Verify) | SCOPE: session | CAP: AP | SEARCHED: 2026-05
// SOURCE: martinfowler.com/articles/harness-engineering.html
//
// Persistence model: append-only event log (kavach-db events table) as source
// of truth for cross-restart resume. Session.toml mirrors hot state for fast reads.
// SOURCE: electric.ax/blog/2026/04/29 — Anthropic Managed Agents pattern.
use crate::cli::LoopAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn run(action: LoopAction) -> i32 {
    match action {
        LoopAction::Start { target, max } => start(&target, max),
        LoopAction::Stop => stop(),
        LoopAction::Status => status(),
    }
}

/// Log loop lifecycle event via kavach-rpc daemon (`SurrealDB`).
/// Failure to log is non-fatal — loop state lives primarily in session.toml.
fn log_loop_event(session_id: &str, event_type: &str, payload: &str) {
    let payload_with_session = format!(r#"{{"sid":"{session_id}","data":{payload}}}"#);
    let params = serde_json::json!({
        "event_type": event_type,
        "source": "harness_loop",
        "project": null,
        "payload": payload_with_session,
    });
    drop(kavach_rpc::client::call::<_, serde_json::Value>(
        "event.append",
        Some(params),
    ));
}

fn start(target: &str, max: i32) -> i32 {
    if !target.starts_with("phase:") && target != "kanban:empty" && target != "goal" {
        let msg =
            format!("error: invalid target '{target}'. Valid: phase:<PHASE>, kanban:empty, goal");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    let mut session = kavach_session::get_or_create_session();
    session.loop_active = true;
    session.loop_target = target.into();
    session.loop_iteration = 0;
    session.loop_max_iterations = max;
    session.loop_start_turn = session.turn_count;
    if let Err(e) = session.save() {
        let msg = format!("error: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    log_loop_event(
        &session.session_id,
        "loop_start",
        &format!(
            r#"{{"target":"{target}","max":{max},"start_turn":{}}}"#,
            session.turn_count
        ),
    );

    let msg = format!("loop started: target={target} max={max}");
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

fn stop() -> i32 {
    let mut session = kavach_session::get_or_create_session();
    if !session.loop_active {
        if let Err(io_err) = print_or_exit("no active loop") {
            return into_exit_code(io_err);
        }
        return 0;
    }

    let iterations = session.loop_iteration;
    let target = session.loop_target.clone();
    session.loop_active = false;
    if let Err(e) = session.save() {
        let msg = format!("error: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    log_loop_event(
        &session.session_id,
        "loop_stop",
        &format!(r#"{{"target":"{target}","iterations":{iterations}}}"#),
    );

    let msg = format!("loop stopped after {iterations} iterations");
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

fn status() -> i32 {
    let mut session = kavach_session::get_or_create_session();

    if let Err(io_err) = print_or_exit("[LOOP_STATUS]") {
        return into_exit_code(io_err);
    }
    let active_line = format!("active: {}", session.loop_active);
    if let Err(io_err) = print_or_exit(&active_line) {
        return into_exit_code(io_err);
    }
    if session.loop_active {
        let target_line = format!("target: {}", session.loop_target);
        if let Err(io_err) = print_or_exit(&target_line) {
            return into_exit_code(io_err);
        }
        let iter_line = format!(
            "iteration: {}/{}",
            session.loop_iteration, session.loop_max_iterations
        );
        if let Err(io_err) = print_or_exit(&iter_line) {
            return into_exit_code(io_err);
        }
        let start_line = format!("start_turn: {}", session.loop_start_turn);
        if let Err(io_err) = print_or_exit(&start_line) {
            return into_exit_code(io_err);
        }
        if session.loop_target == "kanban:empty" {
            session.loop_kanban_runnable =
                kavach_engine::open_set_census(&session.project).map(|(runnable, ..)| runnable);
        }
        let reached_line = format!("target_reached: {}", session.loop_target_reached());
        if let Err(io_err) = print_or_exit(&reached_line) {
            return into_exit_code(io_err);
        }
    }
    0
}
