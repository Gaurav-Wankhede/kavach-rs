// `kavach bg status` — list active bg-session rows via kavach-db query.
// SOURCE: roadmap.unit.kavach-bg-session.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::{Value, json};
use std::fmt::Write as _;

#[expect(
    clippy::arithmetic_side_effects,
    reason = "String capacity pre-allocation bounded by rows count"
)]
pub(crate) fn run(project: &str) -> i32 {
    let params = json!({
        "project": project,
        "category": "decision",
        "key_prefix": "bg.",
    });
    let v = match kavach_rpc::client::call::<Value, Value>("db.list", Some(params)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kavach bg status: rpc db.list: {e}");
            return 1;
        }
    };
    let rows = match v.get("rows").and_then(Value::as_array) {
        Some(r) if !r.is_empty() => r,
        Some(_) | None => {
            return emit_line(&format!(
                "[BG_STATUS] no active bg sessions for project={project}"
            ));
        }
    };
    let mut out = String::with_capacity(64 + rows.len() * 80);
    write!(out, "[BG_STATUS] project={project} active={}", rows.len()).ok();
    for r in rows {
        let key = field(r, "key");
        let title = field(r, "title");
        write!(out, "\n  {key}  {title}").ok();
    }
    emit_line(&out)
}

fn field<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("?")
}

fn emit_line(line: &str) -> i32 {
    if let Err(e) = print_or_exit(line) {
        return into_exit_code(e);
    }
    0
}
