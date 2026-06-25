// `kavach verify` — Writer-Evaluator separation gate (SurrealDB-backed).
// Runs cargo check + nextest, then transitions roadmap entry done → verified.
use std::process::Command;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

mod render;

pub(crate) fn run(
    project_slug: &str,
    key: &str,
    crate_name: Option<&str>,
    external_verified: bool,
    proof: Option<&str>,
) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    runtime.block_on(async {
        run_async(project_slug, key, crate_name, external_verified, proof).await
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "SurrealDB multi-stage verification pipeline with error handling for each stage"
)]
async fn run_async(
    project_slug: &str,
    key: &str,
    crate_name: Option<&str>,
    external_verified: bool,
    proof: Option<&str>,
) -> i32 {
    let db = match kavach_surreal::open_default().await {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("error: open SurrealDB: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let msg = format!("error: project not found: {project_slug}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    if let Err(code) = crate::cmd::db::validate_project_workdir(&project) {
        return code;
    }
    let Some(project_id) = project.id else {
        if let Err(io_err) = ewrite_or_exit("error: project has no id") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let entry = match kavach_surreal::get_by_key(&db, "roadmap", &project_id, key).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            let msg = format!("error: entry not found: {key}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    // Pre-condition: entry must be in 'done' status to be eligible for verify.
    if entry.entry_status_str() != "done" {
        let msg = format!(
            "error: entry '{key}' has status '{}', expected 'done'. \
             Mark it done first: kavach db status-update --status done",
            entry.entry_status_str()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if external_verified {
        let Some(proof_text) = proof.filter(|p| !p.trim().is_empty()) else {
            if let Err(io_err) = ewrite_or_exit(
                "error: --external-verified requires a non-empty --proof (deploy URL / commit / test receipt)",
            ) {
                return into_exit_code(io_err);
            }
            return 1;
        };
        let stamped = format!("{}\n\n[EXTERNAL_VERIFIED] {proof_text}", entry.content);
        let qname = format!("{project_slug}/roadmap/{key}");
        let written = kavach_surreal::upsert_entry_full()
            .db(&db)
            .category("roadmap")
            .project_id(&project_id)
            .entry_key(key)
            .title(&entry.title)
            .content(&stamped)
            .event_source("verify-external")
            .qualified_name(&qname)
            .references(&[])
            .build_for_call()
            .await;
        if let Err(e) = written {
            let msg = format!("error: proof write-back failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        return finalize_verified(&db, &project_id, key).await;
    }

    let head = format!("[VERIFY] roadmap entry: {key}");
    if let Err(io_err) = print_or_exit(&head) {
        return into_exit_code(io_err);
    }
    if let Err(io_err) = print_or_exit("[VERIFY] running: cargo check") {
        return into_exit_code(io_err);
    }

    let mut check_cmd = Command::new("cargo");
    check_cmd.arg("check");
    if let Some(name) = crate_name {
        check_cmd.args(["-p", name]);
    }
    let check_status = match check_cmd.status() {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("error: cargo check failed to spawn: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    if !check_status.success() {
        if let Err(io_err) = ewrite_or_exit("[VERIFY] FAIL: cargo check failed") {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit("[VERIFY] running: cargo nextest run") {
        return into_exit_code(io_err);
    }
    let mut test_cmd = Command::new("cargo");
    test_cmd.args(["nextest", "run"]);
    if let Some(name) = crate_name {
        test_cmd.args(["-p", name]);
    }
    let test_status = match test_cmd.status() {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("error: cargo nextest failed to spawn: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    if !test_status.success() {
        if let Err(io_err) = ewrite_or_exit("[VERIFY] FAIL: cargo nextest failed") {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(e) =
        kavach_surreal::update_status(&db, "roadmap", &project_id, key, "verified").await
    {
        let msg = format!("error: status transition failed: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let pass_msg = format!("[VERIFY] PASS: {key} → verified");
    if let Err(io_err) = print_or_exit(&pass_msg) {
        return into_exit_code(io_err);
    }
    0
}
