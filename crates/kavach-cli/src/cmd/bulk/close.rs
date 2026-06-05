// `kavach bulk close` — terminate a manifest with reason {closed|expired}.
// SOURCE: roadmap.unit.kavach-bulk-mode acceptance #6.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::json;

pub(crate) fn run(sweep_id: &str, reason: &str) -> i32 {
    let params = json!({ "sweep_id": sweep_id, "reason": reason });
    let v = match kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "bulk.sweep_close",
        Some(params),
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kavach bulk close: rpc: {e}");
            return 1;
        }
    };
    let Some(final_status) = v.get("final_status").and_then(serde_json::Value::as_str) else {
        eprintln!("kavach bulk close: rpc returned no final_status");
        return 1;
    };
    let banner = format!(
        "[BULK_SWEEP_CLOSED] sweep_id={sweep_id} final_status={final_status}\n\n\
         NOW unset the env var so per-edit RCA returns to normal:\n  \
         unset KAVACH_BULK_SWEEP_ID"
    );
    if let Err(e) = print_or_exit(&banner) {
        return into_exit_code(e);
    }
    0
}
