// `kavach bulk status` — list active manifests for a project.
// SOURCE: roadmap.unit.kavach-bulk-mode acceptance #6.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::json;
use std::fmt::Write as _;

pub(crate) fn run(project: &str) -> i32 {
    let params = json!({ "project": project });
    let v = match kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "bulk.sweep_list_active",
        Some(params),
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kavach bulk status: rpc: {e}");
            return 1;
        }
    };
    let Some(rows) = v.get("manifests").and_then(serde_json::Value::as_array) else {
        return emit_line("[BULK_STATUS] empty");
    };
    if rows.is_empty() {
        let line = format!("[BULK_STATUS] no active sweeps for project={project}");
        return emit_line(&line);
    }
    let mut out = format!("[BULK_STATUS] project={project} active={}", rows.len());
    for r in rows {
        let sid = field(r, "sweep_id");
        let glob = field(r, "scope_glob");
        let lc = field(r, "lint_class");
        let app = r
            .get("conformance_applied")
            .and_then(serde_json::Value::as_i64);
        let est = r.get("blast_estimate").and_then(serde_json::Value::as_i64);
        let app_s = app.map_or_else(|| "?".to_owned(), |n| n.to_string());
        let est_s = est.map_or_else(|| "?".to_owned(), |n| n.to_string());
        write!(
            out,
            "\n  sweep_id={sid} lint={lc} applied={app_s}/{est_s} glob={glob}"
        )
        .ok();
    }
    emit_line(&out)
}

fn field<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k)
        .and_then(serde_json::Value::as_str)
        .map_or("?", |s| s)
}

fn emit_line(line: &str) -> i32 {
    if let Err(e) = print_or_exit(line) {
        return into_exit_code(e);
    }
    0
}
