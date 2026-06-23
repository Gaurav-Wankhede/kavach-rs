//! `kavach db ope-evaluate` / `ope-audit` — off-policy evaluation + reward-hacking
//! audit front door over the Layer-P5 RL gate. SOURCE: harness-rl P3/P5 commits.
use kavach_rpc::methods::db::{OpeAuditParams, OpeEvaluateParams};

use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{err_exit, into_exit_code, print_or_exit};

/// Pure builder for the candidate-policy evaluation request.
pub(super) const fn build_evaluate_params(
    allow: f64,
    ask: f64,
    block: f64,
    limit: u32,
    z: f64,
    min_coverage_ratio: f64,
) -> OpeEvaluateParams {
    OpeEvaluateParams {
        allow,
        ask,
        block,
        limit,
        z,
        min_coverage_ratio,
    }
}

/// Pure builder for the reward-hacking audit request.
pub(super) const fn build_audit_params(limit: u32, drift_tolerance: f64) -> OpeAuditParams {
    OpeAuditParams {
        limit,
        drift_tolerance,
    }
}

fn emit_json<T: serde::Serialize>(v: &T) -> i32 {
    match serde_json::to_string_pretty(v) {
        Ok(s) => print_or_exit(&s).map_or_else(into_exit_code, |()| 0),
        Err(e) => err_exit(&format!("serialize: {e}")),
    }
}

pub(super) fn run_evaluate(
    allow: f64,
    ask: f64,
    block: f64,
    limit: u32,
    z: f64,
    min_coverage_ratio: f64,
) -> i32 {
    let p = build_evaluate_params(allow, ask, block, limit, z, min_coverage_ratio);
    match rpc_client::ope_evaluate(p) {
        Ok(r) => emit_json(&r),
        Err(e) => err_exit(&format!("ope-evaluate: {e}")),
    }
}

pub(super) fn run_audit(limit: u32, drift_tolerance: f64) -> i32 {
    let p = build_audit_params(limit, drift_tolerance);
    match rpc_client::ope_audit(p) {
        Ok(r) => emit_json(&r),
        Err(e) => err_exit(&format!("ope-audit: {e}")),
    }
}

#[cfg(test)]
#[path = "ope_test.rs"]
mod ope_test;
