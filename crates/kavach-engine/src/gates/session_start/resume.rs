//! Checkpoint resume: re-inject the exact card + harness from last session.
#[cfg(test)]
#[path = "resume_test.rs"]
mod tests;

/// Card statuses meaning "nothing left to resume" — `[KANBAN]` covers the rest.
const CLOSED_STATUSES: [&str; 2] = ["verified", "done"];

/// Pure formatter: `None` on empty key / closed status, else the `[RESUME]` block.
#[must_use]
pub(crate) fn resume_block(key: &str, title: &str, status: &str, harness: &str) -> Option<String> {
    if key.is_empty() || CLOSED_STATUSES.contains(&status) {
        return None;
    }
    let harness_suffix = if harness.is_empty() {
        String::new()
    } else {
        format!(" · harness={harness}")
    };
    Some(format!(
        "[RESUME] you left off on this card last session — claim + continue it:\n\
         · {key} — {title} [{status}]{harness_suffix}\n"
    ))
}

// Impure wrapper: db.get + db.get_harness read-back, fail-soft on any miss.
#[must_use]
pub(in crate::gates) fn resume_context(session: &kavach_session::SessionState) -> Option<String> {
    let key = &session.current_kanban_card;
    if key.is_empty() {
        return None;
    }
    let (title, status) = card_title_status(&session.project, key)?;
    if CLOSED_STATUSES.contains(&status.as_str()) {
        return None;
    }
    let harness = card_harness(&session.project, key).unwrap_or_default();
    resume_block(key, &title, &status, &harness)
}

// `(title, status)` for `(project, key)` via `db.get`, `None` on any miss.
fn card_title_status(project: &str, key: &str) -> Option<(String, String)> {
    let params = serde_json::json!({ "project": project, "category": "roadmap", "key": key });
    let v = kavach_rpc::client::call::<_, serde_json::Value>("db.get", Some(params)).ok()?;
    if v.get("found").and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }
    let entry = v.get("entry")?;
    let title = entry
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let status = entry
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Some((title, status))
}

// Live harness for `(project, key)` via `db.get_harness`, `None` on any miss.
fn card_harness(project: &str, key: &str) -> Option<String> {
    let params = serde_json::json!({ "project": project, "key": key });
    let v = kavach_rpc::client::call::<_, serde_json::Value>("db.get_harness", Some(params)).ok()?;
    v.get("harness")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
