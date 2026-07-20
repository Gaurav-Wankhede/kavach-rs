// Close leaf for `kavach db kanban-close` — marks a roadmap entry verified.
// Tries the RPC daemon first; falls back to a direct resilient SurrealDB open
// only when the daemon is provably down (no competing RocksDB lock holder).
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(in crate::cmd::db) fn close(project_slug: &str, key: &str) -> i32 {
    // EVIDENCE GATE: kanban-close promotes roadmap→verified — run the workspace
    // witness FIRST (mirrors `db status-update`), refuse on failure, else mint a
    // receipt the daemon re-validates.
    let receipt = {
        use kavach_engine::StatusGateVerdict;
        match kavach_engine::verify_status_promotion("roadmap", "verified", "", None) {
            StatusGateVerdict::NotGated | StatusGateVerdict::Allowed => {
                super::super::rpc_client::mint_receipt()
            }
            StatusGateVerdict::RefusedWitnessFailed | StatusGateVerdict::RefusedUnprovable => {
                return write_err(&format!(
                    "REFUSED: cannot close [roadmap] {key}: workspace witnesses FAILED or work \
                     is unprovable. Fix the build/tests (or set KAVACH_VERIFY_CMD), then retry."
                ));
            }
        }
    };
    match super::super::rpc_client::kanban_close(project_slug, key, receipt) {
        Ok(result) if result.success => {
            let ok = format!(
                "closed [roadmap] {} (via rpc daemon)",
                result.title.unwrap_or_else(|| key.to_owned())
            );
            return match print_or_exit(&ok) {
                Ok(()) => 0,
                Err(io_err) => into_exit_code(io_err),
            };
        }
        Ok(result) => {
            let msg = format!(
                "error: {}",
                result.error.unwrap_or_else(|| "unknown".to_owned())
            );
            return write_err(&msg);
        }
        Err(e) if super::super::rpc_client::should_fallback_to_direct(&e) => {
            // Daemon down → no competing RocksDB lock holder → safe to fall
            // through to the direct SurrealDB path below.
        }
        Err(e) => return write_err(&format!("rpc error: {e}")),
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return write_err(&format!("error: tokio runtime: {e}")),
    };
    runtime.block_on(close_direct(project_slug, key))
}

async fn close_direct(project_slug: &str, key: &str) -> i32 {
    let db = match super::super::rpc_client::open_direct_resilient().await {
        Ok(d) => d,
        Err(e) => return write_err(&format!("error: open SurrealDB: {e}")),
    };
    let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
        Ok(Some(p)) => p,
        Ok(None) => return write_err(&format!("error: project not found: {project_slug}")),
        Err(e) => return write_err(&format!("error: {e}")),
    };
    if let Err(code) = super::super::validate_project_workdir(&project) {
        return code;
    }
    let Some(project_id) = project.id else {
        return write_err("error: project has no id");
    };
    let entry = match kavach_surreal::get_by_key(&db, "roadmap", &project_id, key).await {
        Ok(Some(e)) => e,
        Ok(None) => return write_err(&format!("error: no roadmap entry with key: {key}")),
        Err(e) => return write_err(&format!("error: {e}")),
    };
    if let Err(e) =
        kavach_surreal::update_status(&db, "roadmap", &project_id, key, "verified").await
    {
        return write_err(&format!("error: {e}"));
    }
    let ok = format!("closed [roadmap] {key} — {}", entry.title);
    match print_or_exit(&ok) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}

fn write_err(msg: &str) -> i32 {
    if let Err(io_err) = ewrite_or_exit(msg) {
        return into_exit_code(io_err);
    }
    1
}
