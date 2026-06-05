use std::io::{self, Write};

use crate::cli::SessionAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// `kavach session <action>` — manage session lifecycle.
pub(super) fn run(action: &SessionAction) -> i32 {
    match action {
        SessionAction::Init => handle_init(),
        SessionAction::Validate => handle_validate(),
        SessionAction::End => handle_end(),
        SessionAction::Compact => handle_compact(),
        SessionAction::Resume => handle_resume(),
        SessionAction::Land => handle_land(),
        SessionAction::EndHook => handle_end_hook(),
        SessionAction::ClearTestLocks => handle_clear_test_locks(),
    }
}

/// Best-effort blob-write to stdout. Returns the exit code: 0 on success,
/// `EX_IOERR` on broken pipe / EIO. Centralizes the multi-line TOON dump
/// pattern shared by init/validate/compact/resume/land/end.
fn write_blob_to_stdout(blob: &str) -> i32 {
    let mut h = io::stdout().lock();
    if let Err(e) = h.write_all(blob.as_bytes()) {
        return into_exit_code(e.into());
    }
    0
}

fn handle_init() -> i32 {
    let session = kavach_session::get_or_create_session();
    let toon = session.to_compact();
    write_blob_to_stdout(&toon)
}

fn handle_validate() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(session)) => {
            let toon = session.to_compact();
            write_blob_to_stdout(&toon)
        }
        Ok(None) => {
            if let Err(io_err) = ewrite_or_exit("no active session") {
                return into_exit_code(io_err);
            }
            1
        }
        Err(e) => {
            let msg = format!("session error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}

fn handle_end() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(session)) => {
            let research = if session.research_done {
                "DONE"
            } else {
                "PENDING"
            };
            let memory = if session.memory_queried {
                "DONE"
            } else {
                "PENDING"
            };
            let out = format!(
                "[SESSION_END]\n\
                 id: {}\n\
                 project: {}\n\
                 turns: {}\n\
                 research: {research}\n\
                 memory: {memory}\n\
                 files_modified: {}\n\
                 tasks_created: {}\n\
                 tasks_completed: {}\n",
                session.id,
                session.project,
                session.turn_count,
                session.files_modified.len(),
                session.tasks_created,
                session.tasks_completed,
            );
            write_blob_to_stdout(&out)
        }
        Ok(None) => {
            if let Err(io_err) = ewrite_or_exit("no active session to end") {
                return into_exit_code(io_err);
            }
            0
        }
        Err(e) => {
            let msg = format!("session error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}

fn handle_compact() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            session.mark_post_compact();
            let toon = session.to_compact();
            write_blob_to_stdout(&toon)
        }
        Ok(None) => {
            if let Err(io_err) = ewrite_or_exit("no active session to compact") {
                return into_exit_code(io_err);
            }
            1
        }
        Err(e) => {
            let msg = format!("session error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}

fn handle_resume() -> i32 {
    let session = kavach_session::get_or_create_session();
    let toon = session.to_compact();
    write_blob_to_stdout(&toon)
}

fn handle_land() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            session.set_task("", "landed");
            let toon = session.to_compact();
            write_blob_to_stdout(&toon)
        }
        Ok(None) => {
            if let Err(io_err) = ewrite_or_exit("no active session") {
                return into_exit_code(io_err);
            }
            1
        }
        Err(e) => {
            let msg = format!("session error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}

fn handle_clear_test_locks() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            let count = session.active_test_crates.len();
            session.active_test_crates.clear();
            session.save_or_log();
            let msg = format!("cleared {count} stale test lock(s)");
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            0
        }
        Ok(None) => {
            if let Err(io_err) = print_or_exit("no active session") {
                return into_exit_code(io_err);
            }
            0
        }
        Err(e) => {
            let msg = format!("session error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}

fn handle_end_hook() -> i32 {
    if let Ok(Some(session)) = kavach_session::load_session_state() {
        let context = format!(
            "[SESSION_END]\n\
             id: {}\n\
             turns: {}\n\
             files: {}\n",
            session.id,
            session.turn_count,
            session.files_modified.len(),
        );
        drop(kavach_hook::exit_session_end(&context));
    } else {
        drop(kavach_hook::exit_session_end(""));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_accepts_all_session_action_variants() {
        // Compile-time exhaustion check: if a new SessionAction variant is added
        // without a handler arm in run(), this function signature check fails to compile.
        let _: fn(&SessionAction) -> i32 = run;
    }

    #[test]
    fn handle_clear_test_locks_returns_zero_when_no_session() {
        // No active session on disk during unit tests — function must return 0 (not panic).
        let result = handle_clear_test_locks();
        assert_eq!(result, 0);
    }
}
