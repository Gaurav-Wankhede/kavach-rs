use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use crate::cmd::phase_registry;

pub(crate) fn handle_status() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(session)) => {
            let phase = if session.current_phase.is_empty() {
                phase_registry::first()
            } else {
                session.current_phase.clone()
            };
            let iteration = if session.current_iteration_file.is_empty() {
                "(none)"
            } else {
                &session.current_iteration_file
            };
            let done_count = session.iteration_files_done.len();
            let block = format!(
                "[PHASE_STATUS]\n\
                 phase: {phase}\n\
                 phase_start_turn: {}\n\
                 iteration_file: {iteration}\n\
                 files_done_this_phase: {done_count}",
                session.phase_start_turn,
            );
            if let Err(io_err) = print_or_exit(&block) {
                return into_exit_code(io_err);
            }
            0
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

pub(crate) fn handle_advance() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            let current = if session.current_phase.is_empty() {
                phase_registry::first()
            } else {
                session.current_phase.clone()
            };
            if !phase_registry::is_valid(&current) {
                let msg = format!("unknown phase: {current}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            let Some(next) = phase_registry::next_after(&current) else {
                let msg = format!("already at final phase: {current}");
                if let Err(io_err) = print_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 0;
            };
            session.current_phase.clone_from(&next);
            session.phase_start_turn = session.turn_count;
            session.iteration_files_done.clear();
            session.current_iteration_file.clear();
            session.save_or_log();
            let ok = format!("advanced to phase: {next}");
            if let Err(io_err) = print_or_exit(&ok) {
                return into_exit_code(io_err);
            }
            0
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

pub(crate) fn handle_set(phase: &str) -> i32 {
    let phase_upper = phase.to_uppercase();
    if !phase_registry::is_valid(&phase_upper) {
        let msg = format!(
            "invalid phase: {phase}. Valid: {}",
            phase_registry::phases().join(", ")
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            session.current_phase.clone_from(&phase_upper);
            session.phase_start_turn = session.turn_count;
            session.iteration_files_done.clear();
            session.current_iteration_file.clear();
            session.save_or_log();
            let ok = format!("phase set to: {phase_upper}");
            if let Err(io_err) = print_or_exit(&ok) {
                return into_exit_code(io_err);
            }
            0
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_recognizes_all_builtin_phases() {
        for p in ["PLAN", "IMPLEMENT", "TEST", "HARDEN"] {
            assert!(phase_registry::is_valid(p), "registry rejected {p}");
        }
    }
}
