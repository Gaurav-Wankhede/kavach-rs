// ARCH: PhaseCommandHandler — CLI handlers for SDLC phase management
// PATTERN: phase_gate | SCOPE: cli | CAP: AP | SEARCHED: 2026-04
// Per Stanford Meta-Harness: harness sequences enforcement for depth over breadth.

use kavach_session::canonicalize_iteration_path;

use crate::cli::PhaseAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Valid phase names for validation.
const VALID_PHASES: [&str; 4] = ["PLAN", "IMPLEMENT", "TEST", "HARDEN"];

/// `kavach phase <action>` — manage SDLC development phases.
pub(super) fn run(action: PhaseAction) -> i32 {
    match action {
        PhaseAction::Status => handle_status(),
        PhaseAction::Advance => handle_advance(),
        PhaseAction::Set { phase } => handle_set(&phase),
        PhaseAction::IterationStart { file } => handle_iteration_start(&file),
        PhaseAction::IterationDone => handle_iteration_done(),
        PhaseAction::IterationList => handle_iteration_list(),
        PhaseAction::TierSet {
            tier,
            project,
            reason,
            override_flag,
        } => handle_tier_set(&tier, &project, &reason, override_flag),
        PhaseAction::SpikeStart {
            project,
            hours,
            reason,
        } => handle_spike_start(&project, hours, &reason),
        PhaseAction::SpikeEnd { project } => handle_spike_end(&project),
    }
}

fn handle_status() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(session)) => {
            let phase = if session.current_phase.is_empty() {
                "PLAN"
            } else {
                &session.current_phase
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

fn handle_advance() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            let current = if session.current_phase.is_empty() {
                "PLAN"
            } else {
                &session.current_phase
            };
            let next = match current {
                "PLAN" => "IMPLEMENT",
                "IMPLEMENT" => "TEST",
                "TEST" => "HARDEN",
                "HARDEN" => {
                    if let Err(io_err) = print_or_exit("already at final phase: HARDEN") {
                        return into_exit_code(io_err);
                    }
                    return 0;
                }
                _ => {
                    let msg = format!("unknown phase: {current}");
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            };
            // Clear iteration tracking for new phase
            session.current_phase = next.into();
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

fn handle_set(phase: &str) -> i32 {
    let phase_upper = phase.to_uppercase();
    if !VALID_PHASES.contains(&phase_upper.as_str()) {
        let msg = format!("invalid phase: {phase}. Valid: PLAN, IMPLEMENT, TEST, HARDEN");
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

fn handle_iteration_start(file: &str) -> i32 {
    if file.is_empty() {
        if let Err(io_err) = ewrite_or_exit("file path required") {
            return into_exit_code(io_err);
        }
        return 1;
    }
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
            let canonical = canonicalize_iteration_path(file);
            session.current_iteration_file.clone_from(&canonical);
            session.save_or_log();
            let ok = format!("iteration started: {canonical}");
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

fn handle_iteration_done() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(mut session)) => {
            if session.current_iteration_file.is_empty() {
                if let Err(io_err) = ewrite_or_exit("no iteration active") {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            let file = session.current_iteration_file.clone();
            // Move to done list for current phase
            if !session.iteration_files_done.contains(&file) {
                session.iteration_files_done.push(file.clone());
            }
            // Also add to phase-specific done list
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

fn handle_iteration_list() -> i32 {
    match kavach_session::load_session_state() {
        Ok(Some(session)) => {
            let phase = if session.current_phase.is_empty() {
                "PLAN"
            } else {
                &session.current_phase
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

fn handle_tier_set(tier: &str, project: &str, reason: &str, allow_downgrade: bool) -> i32 {
    let tier_lower = tier.to_lowercase();
    let Some(new_tier) = kavach_types::ProjectTier::parse(&tier_lower) else {
        let msg = format!("invalid tier: {tier}. Valid: refactor, feature, platform");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    };

    let current_tier = current_tier_for(project);
    if !current_tier.can_promote_to(new_tier) && !allow_downgrade {
        let msg = format!(
            "[TIER_DOWNGRADE_REFUSED] project={project} current={} target={} \
             — downgrade requires --allow-downgrade (one-way promotion rule)",
            current_tier.as_str(),
            new_tier.as_str()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 2;
    }

    let direction = if current_tier == new_tier {
        "noop"
    } else if current_tier.can_promote_to(new_tier) {
        "promote"
    } else {
        "downgrade"
    };
    let line = format!(
        "[TIER_SET] project={project} {}={}→{} reason={reason}\n\
         Persist with: kavach db write --project {project} --category decision \
         --new --key workflow.tier.current --title 'Project tier: {}' \
         --content 'tier={}; reason={reason}'",
        direction,
        current_tier.as_str(),
        new_tier.as_str(),
        new_tier.as_str(),
        new_tier.as_str(),
    );
    if let Err(io_err) = print_or_exit(&line) {
        return into_exit_code(io_err);
    }
    0
}

// Mirrors six_file::pre_implementation::parse_tier_from_content so both callers
// agree on the `workflow.tier.current` row format (`tier=<value>;reason=...`).
fn parse_tier_line(content: &str) -> Option<kavach_types::ProjectTier> {
    for line in content.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("tier=") else {
            continue;
        };
        let value = match rest.split([';', ' ', '\n']).next() {
            Some(v) => v.trim(),
            None => continue,
        };
        if let Some(tier) = kavach_types::ProjectTier::parse(value) {
            return Some(tier);
        }
    }
    None
}

// Reads workflow.tier.current from kavach-db decisions and parses `tier=<value>`
// from the first matching row. Falls back to Refactor when no row exists, when
// SurrealDB cannot be opened, or when the project slug is unknown — matches the
// six_file::pre_implementation::resolve_project_context default.
fn current_tier_for(project: &str) -> kavach_types::ProjectTier {
    let Ok(runtime) = tokio::runtime::Runtime::new() else {
        return kavach_types::ProjectTier::Refactor;
    };
    runtime.block_on(async { current_tier_for_async(project).await })
}

async fn current_tier_for_async(project: &str) -> kavach_types::ProjectTier {
    let Ok(db) = kavach_surreal::open_default_resilient().await else {
        return kavach_types::ProjectTier::Refactor;
    };
    let Ok(Some(project_rec)) = kavach_surreal::projects::get_by_slug(&db, project).await else {
        return kavach_types::ProjectTier::Refactor;
    };
    let Some(project_id) = project_rec.id else {
        return kavach_types::ProjectTier::Refactor;
    };
    let Ok(rows) = kavach_surreal::read::list_by_project(&db, "decision", &project_id).await else {
        return kavach_types::ProjectTier::Refactor;
    };
    rows.into_iter()
        .find(|row| row.entry_key == "workflow.tier.current")
        .and_then(|row| parse_tier_line(&row.content))
        .map_or(kavach_types::ProjectTier::Refactor, |v| v)
}

fn handle_spike_start(project: &str, hours: u32, reason: &str) -> i32 {
    if hours == 0 {
        if let Err(io_err) = ewrite_or_exit("hours must be > 0") {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).map_or(i64::MAX, |v| v),
        Err(e) => {
            let msg = format!("system clock before UNIX_EPOCH: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let expires_at = now.saturating_add(i64::from(hours).saturating_mul(3600));
    let content =
        format!("started_at_unix_s={now}\nexpires_at_unix_s={expires_at}\nreason={reason}");

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("tokio: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let result: Result<(), String> = runtime.block_on(async {
        let db = kavach_surreal::open_default_resilient()
            .await
            .map_err(|e| format!("open db: {e}"))?;
        let project_rec = kavach_surreal::projects::get_by_slug(&db, project)
            .await
            .map_err(|e| format!("get project: {e}"))?
            .ok_or_else(|| format!("project not found: {project}"))?;
        let pid = project_rec
            .id
            .ok_or_else(|| "project missing id".to_owned())?;
        kavach_surreal::write::upsert_entry_full()
            .db(&db)
            .category("decision")
            .project_id(&pid)
            .entry_key("workflow.spike.active")
            .title("Spike mode active")
            .content(&content)
            .event_source("phase spike-start")
            .qualified_name("")
            .references(&[])
            .build_for_call()
            .await
            .map_err(|e| format!("write spike row: {e}"))?;
        Ok(())
    });
    if let Err(e) = result {
        let msg = format!("spike-start failed: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let ok = format!(
        "[SPIKE_START] project={project} hours={hours} expires_at_unix_s={expires_at} reason={reason}",
    );
    if let Err(io_err) = print_or_exit(&ok) {
        return into_exit_code(io_err);
    }
    0
}

fn handle_spike_end(project: &str) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("tokio: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let result: Result<(), String> = runtime.block_on(async {
        let db = kavach_surreal::open_default_resilient()
            .await
            .map_err(|e| format!("open db: {e}"))?;
        let project_rec = kavach_surreal::projects::get_by_slug(&db, project)
            .await
            .map_err(|e| format!("get project: {e}"))?
            .ok_or_else(|| format!("project not found: {project}"))?;
        let pid = project_rec
            .id
            .ok_or_else(|| "project missing id".to_owned())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).map_or(i64::MAX, |v| v));
        let content = format!("expires_at_unix_s={now}\nreason=spike-ended");
        kavach_surreal::write::upsert_entry_full()
            .db(&db)
            .category("decision")
            .project_id(&pid)
            .entry_key("workflow.spike.active")
            .title("Spike mode ended")
            .content(&content)
            .event_source("phase spike-end")
            .qualified_name("")
            .references(&[])
            .build_for_call()
            .await
            .map_err(|e| format!("write spike-end: {e}"))?;
        Ok(())
    });
    if let Err(e) = result {
        let msg = format!("spike-end failed: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let ok = format!("[SPIKE_END] project={project}");
    if let Err(io_err) = print_or_exit(&ok) {
        return into_exit_code(io_err);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_accepts_all_phase_action_variants() {
        let _: fn(PhaseAction) -> i32 = run;
    }

    #[test]
    fn valid_phases_contains_all_expected() {
        assert!(VALID_PHASES.contains(&"PLAN"));
        assert!(VALID_PHASES.contains(&"IMPLEMENT"));
        assert!(VALID_PHASES.contains(&"TEST"));
        assert!(VALID_PHASES.contains(&"HARDEN"));
    }

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
