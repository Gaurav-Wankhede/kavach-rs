//! `[PATTERN_DAG]` intent-time injector — the research-refreshed pattern layer's
//! supersession DAG as Mermaid, so the model sees which detector/boundary THIS
//! codebase has already retired (`-.retires.->`) and which is current, plus a
//! `(fresh)` tag on patterns not yet soaked. The legend + version-scheme
//! stability rule ride the `directive_cache` substrate (DB-cached, Haiku-
//! refreshed, `[STALE]`-marked, fail-soft to literal), so the pattern guidance
//! re-researches itself as ecosystems move. Read-side VIEW over the pattern
//! graph; fail-soft `None`. SOURCE: roadmap.unit.mermaid-decision-architecture.

use crate::gates::directive_cache::dyn_directive;

/// Literal served when the cache has no `pattern-dag.legend` row yet.
const LEGEND_FALLBACK: &str = "`-.retires.->` = source replaced target; `(fresh)` = not yet soaked";

/// Literal served when the cache has no `pattern-dag.stability` row yet. The
/// version-scheme rule is tech-stack-agnostic: the breaking-change axis is read
/// off the version itself (0.x ⇒ MINOR bump breaks; 1.x+ ⇒ MAJOR bump breaks).
const STABILITY_FALLBACK: &str = "0.x ⇒ MINOR bump breaks; 1.x+ ⇒ MAJOR bump breaks";

/// Build the `[PATTERN_DAG]` block for `project_slug`, focused on the pattern
/// keys most relevant to `prompt` (via Brain-OS) so only the touched
/// neighbourhood is shown. `None` when the project has no pattern rows, the
/// daemon is down, or a real prompt yields no relevant pattern keys (the
/// context-rot guard: don't whole-spine on a thin hit).
#[must_use]
pub(in crate::gates) fn pattern_dag_block(project_slug: &str, prompt: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let focus = relevant_pattern_keys(prompt);
    if !prompt.trim().is_empty() && focus.is_empty() {
        return None;
    }
    let params =
        serde_json::json!({ "project_slug": project_slug, "focus": focus, "max_nodes": 8 });
    let res: kavach_rpc::methods::db::PatternRenderResult =
        kavach_rpc::client::call("db.pattern_render", Some(params)).ok()?;
    let mermaid = res.mermaid?;
    if mermaid.trim().is_empty() {
        return None;
    }
    let legend = dyn_directive("pattern-dag.legend", LEGEND_FALLBACK);
    let stability = dyn_directive("pattern-dag.stability", STABILITY_FALLBACK);
    Some(format!(
        "\n[PATTERN_DAG] patterns ({legend}):\n\
         ```mermaid\n{}\n```\n{stability}",
        mermaid.trim_end()
    ))
}

/// Brain-OS-ranked pattern keys relevant to the prompt, so the DAG is scoped to
/// the touched neighbourhood. Empty (⇒ caller decides whole-spine vs none) on any
/// RPC error or empty prompt.
fn relevant_pattern_keys(prompt: &str) -> Vec<String> {
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
        .filter(|id| id.starts_with("pattern."))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_yields_none() {
        assert!(pattern_dag_block("", "anything").is_none());
    }

    #[test]
    fn real_prompt_with_no_daemon_yields_none() {
        // Non-empty prompt + no RPC server ⇒ empty focus ⇒ context-rot guard
        // returns None (never degrades to whole-spine).
        assert!(pattern_dag_block("kavach-rs", "add auth").is_none());
    }

    #[test]
    fn block_is_none_or_well_formed() {
        // Empty prompt = session-start whole-spine path (focus guard inactive).
        match pattern_dag_block("kavach-rs", "") {
            None => {}
            Some(b) => {
                assert!(b.contains("[PATTERN_DAG]"), "{b}");
                assert!(b.contains("graph TD"), "{b}");
                // Both directive-cached strands surface (literal fallback in
                // unit context, since no RPC server ⇒ cache absent).
                assert!(b.contains("`-.retires.->`"), "legend present: {b}");
                assert!(b.contains("MINOR bump breaks"), "stability: {b}");
            }
        }
    }

    // The version-scheme rule is stack-agnostic: it names the 0.x vs 1.x+
    // breaking-change axis without hardcoding any language or framework.
    #[test]
    fn stability_fallback_is_techstack_agnostic() {
        let f = STABILITY_FALLBACK.to_lowercase();
        assert!(f.contains("0.x") && f.contains("1.x"), "{f}");
        for stack in ["rust", "dioxus", "react", "npm", "cargo", "python"] {
            assert!(!f.contains(stack), "names a specific stack: {stack}");
        }
    }
}
