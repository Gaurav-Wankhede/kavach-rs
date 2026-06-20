//! `[DECISION_MAP]` intent-time injector — the project's decision architecture as
//! a Mermaid `graph TD`, so the model reads settled choices + their constraints as
//! hard edges instead of hallucinating them when the architecture is complex.
//! Read-side VIEW over the decision/roadmap DAG; fail-soft `None` (daemon down or
//! no decisions ⇒ inject nothing). SOURCE: roadmap.unit.mermaid-decision-architecture.

/// Build the `[DECISION_MAP]` block for `project_slug`, focused on the keys most
/// relevant to `prompt` (via Brain-OS) so only the touched neighbourhood is shown.
/// `None` when the project has no decision/roadmap rows or the daemon is down.
#[must_use]
pub(super) fn decision_map_block(project_slug: &str, prompt: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let focus = relevant_keys(prompt);
    let params = serde_json::json!({
        "project_slug": project_slug,
        "focus": focus,
        "max_nodes": 8,
    });
    let res: kavach_rpc::methods::db::DecisionRenderResult =
        kavach_rpc::client::call("db.decision_render", Some(params)).ok()?;
    let mermaid = res.mermaid?;
    if mermaid.trim().is_empty() {
        return None;
    }
    Some(format!(
        "\n[DECISION_MAP] this project's decision architecture (settled choices are \
         hard constraints — do NOT contradict a VERIFIED decision; `-.retires.->` \
         means the target was replaced, do not reintroduce it):\n\
         ```mermaid\n{}\n```\n\
         apply: build consistent with these edges; if your change requires breaking \
         one, FILE a superseding decision row first.",
        mermaid.trim_end()
    ))
}

/// Brain-OS-ranked decision/roadmap keys relevant to the prompt, so the map is
/// scoped to the touched neighbourhood. Empty (⇒ whole spine) on any RPC error.
fn relevant_keys(prompt: &str) -> Vec<String> {
    if prompt.trim().is_empty() {
        return Vec::new();
    }
    let params = serde_json::json!({ "query": prompt, "limit": 8 });
    let hits: Vec<kavach_surreal::BrainHit> =
        match kavach_rpc::client::call("brain.think", Some(params)) {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };
    hits.into_iter()
        .map(|h| h.id)
        .filter(|id| id.starts_with("decision.") || id.starts_with("roadmap."))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_yields_none() {
        assert!(decision_map_block("", "anything").is_none());
    }

    #[test]
    fn no_daemon_yields_none() {
        // No RPC server in unit-test ⇒ call errors ⇒ None (fail-soft).
        assert!(decision_map_block("kavach-rs", "add auth").is_none());
    }
}
