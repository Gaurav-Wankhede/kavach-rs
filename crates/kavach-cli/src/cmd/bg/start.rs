// `kavach bg start` — declare a /bg background-session intent + persist row.
// SOURCE: roadmap.unit.kavach-bg-session.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::{Value, json};

pub(crate) fn run(project: &str, task: &str, isolation: &str) -> i32 {
    let key = format!("bg.{}", slugify(task));

    // Guard: check if this bg task is already running.
    if let Err(e) = check_not_running(project, &key) {
        eprintln!("kavach bg start: {e}");
        return 1;
    }

    let content = format!("task: {task}\nisolation: {isolation}\nstatus: active");
    let params = json!({
        "project": project,
        "category": "decision",
        "key": key,
        "title": format!("Bg: {task}"),
        "content": content,
    });
    if let Err(e) = kavach_rpc::client::call::<Value, Value>("db.write", Some(params)) {
        eprintln!("kavach bg start: rpc db.write: {e}");
        return 1;
    }
    let banner = format!(
        "[BG_DECLARED] project={project} task={task} isolation={isolation}\n\n\
         PASTE INTO CLAUDE CODE NOW:\n  \
         /bg work on {task}\n\n\
         When the bg session reports done, run:\n  \
         kavach bg stop --project {project} --task {task:?}"
    );
    if let Err(e) = print_or_exit(&banner) {
        return into_exit_code(e);
    }
    0
}

/// Check whether a bg task keyed `<key>` is already running in `<project>`.
/// Returns Ok(()) if not running, Err(msg) if collision detected or RPC fails.
fn check_not_running(project: &str, key: &str) -> Result<(), String> {
    let params = json!({
        "project": project,
        "category": "decision",
        "key": key,
    });
    let resp = kavach_rpc::client::call::<Value, Value>("db.get", Some(params))
        .map_err(|e| format!("db.get failed: {e}"))?;

    // db.get returns { found: bool, entry: { status: ..., ... } | null }
    let found = resp.get("found").and_then(Value::as_bool).unwrap_or(false);
    if !found {
        return Ok(());
    }

    let entry = resp.get("entry");
    let status = entry
        .and_then(|e| e.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("");

    match status {
        "active" => Err(format!(
            "background task '{key}' is already running (use `kavach bg stop --project {project} --task <name>` first)"
        )),
        _ => Ok(()), // task exists but is done or in other state; allow reuse
    }
}

pub(super) fn slugify_for_test(s: &str) -> String {
    slugify(s)
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(48));
    let mut last_dash = false;
    for c in s.chars().take(48) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_end_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::slugify;
    use serde_json::json;

    #[test]
    fn slugify_lowercases_alnum() {
        assert_eq!(slugify("Roadmap.Unit.X"), "roadmap-unit-x");
    }
    #[test]
    fn slugify_caps_length() {
        let s = "a".repeat(100);
        assert!(slugify(&s).len() <= 48);
    }

    #[test]
    fn is_running_collision_detects_active() {
        let active_response = json!({
            "found": true,
            "entry": {
                "key": "bg.deploy-api",
                "title": "Bg: deploy api",
                "status": "active",
                "content": "...",
                "access_count": 0
            }
        });
        let is_collision = is_running_collision(&active_response);
        assert!(is_collision, "should detect active status as collision");
    }

    #[test]
    fn is_running_collision_allows_done() {
        let done_response = json!({
            "found": true,
            "entry": {
                "key": "bg.deploy-api",
                "title": "Bg: deploy api",
                "status": "done",
                "content": "...",
                "access_count": 0
            }
        });
        let is_collision = is_running_collision(&done_response);
        assert!(!is_collision, "should allow reuse of done task");
    }

    #[test]
    fn is_running_collision_allows_not_found() {
        let not_found_response = json!({
            "found": false,
            "entry": null
        });
        let is_collision = is_running_collision(&not_found_response);
        assert!(!is_collision, "should allow new task when key not found");
    }

    /// Pure decision: is the db.get response a collision (already running)?
    fn is_running_collision(resp: &serde_json::Value) -> bool {
        let found = resp
            .get("found")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !found {
            return false;
        }
        let status = resp
            .get("entry")
            .and_then(|e| e.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        status == "active"
    }
}
