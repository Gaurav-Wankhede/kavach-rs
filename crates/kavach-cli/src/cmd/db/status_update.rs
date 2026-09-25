// `kavach db status-update` — set strict entry_status for a memory entry.
// SurrealDB-backed: routes through kavach_db::update_status with table name.
// SOURCE: https://docs.rs/strum/0.28 — MemoryStatus uses strum EnumString + EnumIter.
use kavach_types::MemoryStatus;
use std::str::FromStr as _;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::turso::entries::EntryStore;

#[expect(
    clippy::too_many_lines,
    reason = "RPC-first with fallback to direct DB requires nested match arms and error handling"
)]
pub(super) fn run(
    project_slug: &str,
    category: &str,
    key: &str,
    status: &str,
    verify_cmd: Option<&str>,
) -> i32 {
    if MemoryStatus::from_str(status).is_err() {
        let msg = format!(
            "error: invalid status '{status}'. Valid: {}",
            MemoryStatus::allowed_list()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // EVIDENCE GATE (operator directive 2026-06-18): a roadmap promotion to
    // `done`/`verified` is REFUSED unless the objective workspace witnesses
    // (cargo check+clippy+nextest+diff, or KAVACH_VERIFY_CMD) pass NOW. This binds
    // proof to the claim at the agent-facing entry point — the DB rejects a
    // completion claim that the build does not support (the false-`done` hole).
    {
        use kavach_engine::StatusGateVerdict;
        // SINGLE-CARD INVARIANT: Prevent bulk-verification of multiple cards without
        // individual task execution. Exactly one card can be promoted per pass.
        match kavach_engine::verify_status_promotion(category, status, "", verify_cmd) {
            StatusGateVerdict::NotGated | StatusGateVerdict::Allowed => {}
            StatusGateVerdict::RefusedWitnessFailed => {
                let msg = format!(
                    "REFUSED: cannot promote [{category}] {key} -> {status}: workspace witnesses \
                     FAILED (build/clippy/nextest/diff). Fix the implementation until it builds \
                     and tests pass, then retry. (evidence-over-inference: a `{status}` claim must \
                     be backed by a passing build, not self-report.)"
                );
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            StatusGateVerdict::RefusedUnprovable => {
                let msg = format!(
                    "REFUSED: cannot promote [{category}] {key} -> {status}: work is UNPROVABLE \
                     here (no Rust workspace and no --verify-cmd/KAVACH_VERIFY_CMD). Pass \
                     --verify-cmd '<command>' to prove the work or set KAVACH_VERIFY_CMD."
                );
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        }
    }

    // The witness gate above PASSED (or was NotGated) — mint a receipt so the
    // daemon's RPC-boundary gate accepts the promotion (it re-validates HEAD+freshness).
    let receipt = super::rpc_client::mint_receipt();
    // RPC-first: try daemon, fall back to direct DB on unavailable
    match super::rpc_client::status_update(project_slug, category, key, status, Some(receipt)) {
        Ok(result) if result.success => {
            let msg = format!("status applied: [{category}] {key} -> {status} (via rpc)");
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(result) => {
            let err_msg = result.error.unwrap_or_else(|| "unknown".to_owned());
            let msg = format!("error: {err_msg}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            let msg = format!("rpc error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

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
        // Phase 6: direct open via TursoStore (libSQL file — no lock contention).
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        // Use CAS update for safety - only update if current status matches expected
        // For now, we'll use the simple update_status (non-CAS) since we don't have the current status
        match store
            .update_status(project_slug, category, key, status)
            .await
        {
            Ok(n) if n > 0 => {
                let msg = format!("status: [{category}] {key} → {status}");
                if let Err(io_err) = print_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                0
            }
            Ok(_) => {
                let msg = format!("error: entry not found: {category}/{key}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
        }
    })
}
