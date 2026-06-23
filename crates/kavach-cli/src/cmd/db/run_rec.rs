//! `kavach db run-record` / `run-update-status` — front door over the run-row
//! lifecycle RPC (records a harness execution + its terminal status).
use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{err_exit, into_exit_code, print_or_exit};

/// Pure builder for `run.record` params (None → JSON null).
pub(super) fn build_record_params(
    project: &str,
    entry_key: &str,
    branch: Option<&str>,
    status: &str,
    pid: Option<i64>,
) -> serde_json::Value {
    serde_json::json!({
        "project": project,
        "entry_key": entry_key,
        "branch": branch,
        "status": status,
        "pid": pid,
    })
}

/// Pure builder for `run.update_status` params.
pub(super) fn build_update_params(id: &str, status: &str, exit_code: Option<i64>) -> serde_json::Value {
    serde_json::json!({ "id": id, "status": status, "exit_code": exit_code })
}

fn emit(v: &serde_json::Value) -> i32 {
    print_or_exit(&v.to_string()).map_or_else(into_exit_code, |()| 0)
}

pub(super) fn run_record(
    project: &str,
    entry_key: &str,
    branch: Option<&str>,
    status: &str,
    pid: Option<i64>,
) -> i32 {
    let params = build_record_params(project, entry_key, branch, status, pid);
    match rpc_client::run_record(params) {
        Ok(v) => emit(&v),
        Err(e) => err_exit(&format!("run-record: {e}")),
    }
}

pub(super) fn run_update_status(id: &str, status: &str, exit_code: Option<i64>) -> i32 {
    let params = build_update_params(id, status, exit_code);
    match rpc_client::run_update_status(params) {
        Ok(v) => emit(&v),
        Err(e) => err_exit(&format!("run-update-status: {e}")),
    }
}

#[cfg(test)]
#[path = "run_rec_test.rs"]
mod run_rec_test;
