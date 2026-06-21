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
const LEGEND_FALLBACK: &str = "`-.retires.->` means the source pattern replaced \
    the target — use the current one, not the retired; `(fresh)` marks a pattern \
    adopted but not yet soaked, so prefer a verified sibling when one exists";

/// Literal served when the cache has no `pattern-dag.stability` row yet. The
/// version-scheme rule is tech-stack-agnostic: the breaking-change axis is read
/// off the version itself (0.x ⇒ MINOR bump breaks; 1.x+ ⇒ MAJOR bump breaks).
const STABILITY_FALLBACK: &str = "version-scheme stability: for a 0.x dependency a \
    MINOR bump (0.7→0.8) is BREAKING; for 1.x+ only a MAJOR bump breaks — when a \
    superseding pattern names a newer version, adopt it only after confirming the \
    bump's stability on that axis";

/// Build the `[PATTERN_DAG]` block for `project_slug`. `None` when the project
/// has no pattern rows or the daemon is down.
#[must_use]
pub(in crate::gates) fn pattern_dag_block(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({ "project_slug": project_slug, "max_nodes": 8 });
    let res: kavach_rpc::methods::db::PatternRenderResult =
        kavach_rpc::client::call("db.pattern_render", Some(params)).ok()?;
    let mermaid = res.mermaid?;
    if mermaid.trim().is_empty() {
        return None;
    }
    let legend = dyn_directive("pattern-dag.legend", LEGEND_FALLBACK);
    let stability = dyn_directive("pattern-dag.stability", STABILITY_FALLBACK);
    Some(format!(
        "\n[PATTERN_DAG] research-refreshed pattern layer for this project ({legend}):\n\
         ```mermaid\n{}\n```\n{stability}",
        mermaid.trim_end()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_yields_none() {
        assert!(pattern_dag_block("").is_none());
    }

    #[test]
    fn block_is_none_or_well_formed() {
        match pattern_dag_block("kavach-rs") {
            None => {}
            Some(b) => {
                assert!(b.contains("[PATTERN_DAG]"), "{b}");
                assert!(b.contains("graph TD"), "{b}");
                // Both directive-cached strands surface (literal fallback in
                // unit context, since no RPC server ⇒ cache absent).
                assert!(b.contains("`-.retires.->`"), "legend present: {b}");
                assert!(b.contains("version-scheme stability"), "stability: {b}");
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
