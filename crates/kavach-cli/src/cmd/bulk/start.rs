// `kavach bulk start` — create a manifest after [RCA] validation.
// Reads --rca-file (must contain "[RCA]" header) and posts bulk.sweep_create.
// SOURCE: roadmap.unit.kavach-bulk-mode acceptance #6.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::json;

#[derive(Debug)]
pub(crate) struct StartParams<'a> {
    pub sweep_id: &'a str,
    pub project: &'a str,
    pub rca_file: &'a str,
    pub scope_glob: &'a str,
    pub lint_class: &'a str,
    pub fix_strategy: &'a str,
    pub blast_estimate: i64,
    pub ttl_seconds: i64,
    pub approved_by: &'a str,
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "dispatch entrypoint: StartParams is a borrow-bundle handed off by the bulk action match; by-value keeps the call site a single struct literal"
)]
pub(crate) fn run(p: StartParams<'_>) -> i32 {
    let rca = match std::fs::read_to_string(p.rca_file) {
        Ok(s) if s.contains("[RCA]") => s,
        Ok(_) => {
            eprintln!("kavach bulk start: --rca-file must contain an [RCA] block");
            return 1;
        }
        Err(e) => {
            eprintln!("kavach bulk start: read rca_file: {e}");
            return 1;
        }
    };
    let session_id = match std::env::var("CLAUDE_SESSION_ID") {
        Ok(s) if !s.is_empty() => s,
        Ok(_) | Err(std::env::VarError::NotPresent) => "unknown-session".to_owned(),
        Err(std::env::VarError::NotUnicode(_)) => {
            eprintln!("kavach bulk start: CLAUDE_SESSION_ID not valid unicode");
            return 1;
        }
    };
    let params = json!({
        "sweep_id": p.sweep_id, "project": p.project, "root_rca": rca,
        "scope_glob": p.scope_glob, "lint_class": p.lint_class,
        "fix_strategy": p.fix_strategy, "blast_estimate": p.blast_estimate,
        "signed_by_session": session_id, "approved_by": p.approved_by,
        "ttl_seconds": p.ttl_seconds,
    });
    match kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "bulk.sweep_create",
        Some(params),
    ) {
        Ok(v) => emit_started_banner(&p, &v),
        Err(e) => {
            eprintln!("kavach bulk start: rpc: {e}");
            1
        }
    }
}

fn emit_started_banner(p: &StartParams<'_>, v: &serde_json::Value) -> i32 {
    let Some(expires) = v.get("expires_at").and_then(serde_json::Value::as_str) else {
        eprintln!("kavach bulk start: rpc returned no expires_at");
        return 1;
    };
    let banner = format!(
        "[BULK_SWEEP_STARTED] sweep_id={sid} project={proj}\n  \
         scope_glob={glob}\n  lint_class={lc}\n  blast_estimate={est}\n  \
         ttl_seconds={ttl}\n  expires_at={expires}\n\n\
         NOW export the env var so the gate short-circuits per-Edit RCA:\n  \
         export KAVACH_BULK_SWEEP_ID={sid}\n\n\
         When done: kavach bulk close --sweep-id {sid} --reason closed",
        sid = p.sweep_id,
        proj = p.project,
        glob = p.scope_glob,
        lc = p.lint_class,
        est = p.blast_estimate,
        ttl = p.ttl_seconds,
    );
    if let Err(e) = print_or_exit(&banner) {
        return into_exit_code(e);
    }
    0
}
