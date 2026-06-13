//! `[CONCEPT]` session-start read path — recently-bridged L0 concepts for a project.
use std::collections::HashSet;
use std::fmt::Write as _;

use kavach_surreal::BridgeHit;

/// Max distinct concepts injected at session start (token discipline).
const CONCEPT_TOP_K: usize = 5;

/// Load bridged L0 concepts via `bridge.concepts_for` and format for injection.
#[must_use]
pub(super) fn concept_context(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({"project": project_slug});
    let hits: Vec<BridgeHit> =
        kavach_rpc::client::call("bridge.concepts_for", Some(params)).ok()?;
    if hits.is_empty() {
        return None;
    }
    let mut seen = HashSet::new();
    let mut ctx = String::from("\n[CONCEPT] recently learned (L0, project-bridged)\n");
    let mut count = 0usize;
    for hit in hits {
        let name = hit.concept.name.trim();
        if name.is_empty() || !seen.insert(name.to_owned()) {
            continue;
        }
        let desc = concept_description(&hit);
        if desc.is_empty() {
            writeln!(ctx, "• {name}").ok();
        } else {
            writeln!(ctx, "• {name}: {desc}").ok();
        }
        count = count.saturating_add(1);
        if count >= CONCEPT_TOP_K {
            break;
        }
    }
    if count == 0 {
        return None;
    }
    ctx.push_str("apply: use these concepts when scoping work — do not ignore fresh L0 knowledge.\n");
    Some(ctx)
}

fn concept_description(hit: &BridgeHit) -> String {
    hit.concept
        .properties
        .as_ref()
        .and_then(|p| p.get("description").or_else(|| p.get("desc")))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_returns_none() {
        assert!(concept_context("").is_none());
    }
}
