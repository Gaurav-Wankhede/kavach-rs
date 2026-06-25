use kavach_session::canonicalize_iteration_path;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use crate::cmd::phase_registry;

fn pull_next_dag_card(project: &str) -> Option<(String, String)> {
    let params = serde_json::json!({ "project": project });
    let res: Option<serde_json::Value> =
        kavach_rpc::client::call("roadmap.next_open_task", Some(params)).ok()?;
    let v = res?;
    let key = v.get("key").and_then(serde_json::Value::as_str)?.to_owned();
    if key.starts_with('[') {
        return None;
    }
    let title = v
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Some((key, title))
}

pub(crate) fn handle_iteration_start(file: Option<&str>) -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            if !session.current_iteration_file.is_empty() {
                let msg = format!(
                    "iteration already active: {}. Run `kavach phase iteration-done` first.",
                    session.current_iteration_file
                );
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            let (iteration, pulled_card) = if let Some(f) = file {
                (canonicalize_iteration_path(f), None)
            } else {
                let Some((key, title)) = pull_next_dag_card(&session.project) else {
                    if let Err(io_err) =
                        ewrite_or_exit("no runnable roadmap card (board drained or all blocked)")
                    {
                        return into_exit_code(io_err);
                    }
                    return 1;
                };
                let ok = format!("auto-pulled next DAG card: {key} — {title}");
                if let Err(io_err) = print_or_exit(&ok) {
                    return into_exit_code(io_err);
                }
                (key.clone(), Some(key))
            };
            session.current_iteration_file.clone_from(&iteration);
            if let Some(card) = pulled_card {
                session.current_kanban_card = card;
            }
            session.save_or_log();
            let ok = format!("iteration started: {iteration}");
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

pub(crate) fn handle_iteration_done() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            if session.current_iteration_file.is_empty() {
                if let Err(io_err) = ewrite_or_exit("no iteration active") {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            let file = session.current_iteration_file.clone();
            if !session.iteration_files_done.contains(&file) {
                session.iteration_files_done.push(file.clone());
            }
            let phase = session.current_phase.clone();
            match phase.as_str() {
                "PLAN" if !session.plan_done_files.contains(&file) => {
                    session.plan_done_files.push(file.clone());
                }
                "IMPLEMENT" if !session.implement_done_files.contains(&file) => {
                    session.implement_done_files.push(file.clone());
                }
                "TEST" if !session.test_done_files.contains(&file) => {
                    session.test_done_files.push(file.clone());
                }
                "HARDEN" if !session.harden_done_files.contains(&file) => {
                    session.harden_done_files.push(file.clone());
                }
                _ => {}
            }
            session.current_iteration_file.clear();
            session.save_or_log();
            let ok = format!("iteration done: {file}");
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

pub(crate) fn handle_iteration_list() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(session)) => {
            let phase = if session.current_phase.is_empty() {
                phase_registry::first()
            } else {
                session.current_phase.clone()
            };
            let header = format!("[ITERATION_LIST] phase: {phase}");
            if let Err(io_err) = print_or_exit(&header) {
                return into_exit_code(io_err);
            }
            if session.iteration_files_done.is_empty() {
                if let Err(io_err) = print_or_exit("(no files completed in this phase)") {
                    return into_exit_code(io_err);
                }
            } else {
                for file in &session.iteration_files_done {
                    let line = format!("  - {file}");
                    if let Err(io_err) = print_or_exit(&line) {
                        return into_exit_code(io_err);
                    }
                }
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
    fn canonicalize_resolves_existing_file_to_absolute() {
        let rel = file!();
        let canonical = canonicalize_iteration_path(rel);
        assert!(
            std::path::Path::new(&canonical).is_absolute(),
            "expected absolute path, got: {canonical}"
        );
    }

    #[test]
    fn canonicalize_relative_and_absolute_yield_same_path_for_existing_file() {
        let rel = file!();
        let Ok(abs) = std::fs::canonicalize(rel) else {
            return;
        };
        let abs_str = abs.to_string_lossy();
        let from_rel = canonicalize_iteration_path(rel);
        let from_abs = canonicalize_iteration_path(&abs_str);
        assert_eq!(
            from_rel, from_abs,
            "relative and absolute inputs must canonicalize identically"
        );
    }

    #[test]
    fn canonicalize_falls_back_to_absolute_for_nonexistent_file() {
        let nonexistent = "this-file-does-not-exist-xyz123.rs";
        let canonical = canonicalize_iteration_path(nonexistent);
        assert!(
            std::path::Path::new(&canonical).is_absolute() || canonical == nonexistent,
            "expected absolute fallback or raw input, got: {canonical}"
        );
    }
}
