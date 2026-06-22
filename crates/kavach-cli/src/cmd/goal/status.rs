// `kavach goal status` — list active goals via kavach-db query.
// SOURCE: roadmap.unit.kavach-goal-bridge.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::{Value, json};
use std::fmt::Write as _;

pub(crate) fn run(project: &str) -> i32 {
    let params = json!({
        "project": project,
        "category": "decision",
    });
    let v = match kavach_rpc::client::call::<Value, Value>("db.query", Some(params)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kavach goal status: rpc db.query: {e}");
            return 1;
        }
    };
    // db.query returns `.entries`; filter the `goal.` key prefix client-side.
    let all: Vec<Value> = v
        .get("entries")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|e| field(e, "key").starts_with("goal."))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let rows = match all.as_slice() {
        r if !r.is_empty() => r,
        _ => {
            return emit_line(&format!(
                "[GOAL_STATUS] no active goals for project={project}"
            ));
        }
    };
    let mut out = String::with_capacity(64_usize.saturating_add(rows.len().saturating_mul(80)));
    write!(out, "[GOAL_STATUS] project={project} active={}", rows.len()).ok();
    for r in rows {
        let key = field(r, "key");
        let title = field(r, "title");
        write!(out, "\n  {key}  {title}").ok();
    }
    emit_line(&out)
}

fn field<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).map_or("?", |s| s)
}

fn emit_line(line: &str) -> i32 {
    if let Err(e) = print_or_exit(line) {
        return into_exit_code(e);
    }
    0
}
