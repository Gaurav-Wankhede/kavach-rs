// TIME: O(1) avg | SPACE: O(1)
//! db.write RPC method — create or update entry.

use super::util::resolve_project_id;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_types::Priority;
use serde::{Deserialize, Serialize};

mod relationships;

const ERR_BOTH: &str = "'new' and 'update_key' are mutually exclusive";
const ERR_NEITHER: &str = "must specify 'new: true' or 'update_key'";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct WriteParams {
    pub project: String,
    pub category: String,
    pub key: String,
    pub title: String,
    pub content: Option<String>,
    #[serde(default)]
    pub new: Option<bool>,
    pub update_key: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    /// Opus-authored executor prompt (roadmap only); persisted for `db next-prompt`.
    #[serde(default)]
    pub exec_prompt: Option<String>,
    /// Fully-resolved inter-entry edges `(rel, target_qname)` the CLI extracted
    /// from body (frontmatter/wikilink/NLU) merged with `--depends-on`. The CLI
    /// owns extraction (it depends on `kavach-engine`; the daemon cannot — that
    /// would cycle); the daemon — the single `RocksDB` writer — owns projection.
    #[serde(default)]
    pub relationships: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct WriteResult {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}

/// Mirror `depends_on` edge targets from `relationships` into a `DEPENDS_ON:`
/// content line so the dispatch readiness check (which parses deps from CONTENT)
/// honors them. Returns `content` unchanged when there are no `depends_on` edges.
/// Idempotent: a target already named on a `DEPENDS_ON:`/`BLOCKED_BY:` content
/// line is not re-added, so re-running an update never duplicates the line.
fn mirror_depends_on_into_content(content: &str, relationships: &[(String, String)]) -> String {
    let fresh: Vec<&str> = relationships
        .iter()
        .filter(|(rel, _)| rel == "depends_on")
        .map(|(_, target)| target.trim())
        .filter(|t| !t.is_empty())
        .filter(|t| !content_declares_dep(content, t))
        .collect();
    if fresh.is_empty() {
        return content.to_owned();
    }
    let line = format!("DEPENDS_ON: {}", fresh.join(", "));
    if content.is_empty() {
        line
    } else {
        format!("{line}\n{content}")
    }
}

/// `true` iff `content` already names `dep` on a `DEPENDS_ON:`/`BLOCKED_BY:` line
/// — the same lines the readiness parser reads. Whitespace/comma tolerant.
fn content_declares_dep(content: &str, dep: &str) -> bool {
    content
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("DEPENDS_ON:") || l.starts_with("BLOCKED_BY:"))
        .any(|l| {
            l.split([':', ',', ' ', '\t'])
                .map(str::trim)
                .any(|tok| tok == dep)
        })
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when validation fails or database write fails.
pub async fn write(ctx: &AppState, params: WriteParams) -> Result<WriteResult, ErrorObjectOwned> {
    let is_new = params.new.unwrap_or_default();
    if !is_new && params.update_key.is_none() {
        return Ok(WriteResult {
            success: false,
            id: None,
            error: Some(ERR_NEITHER.to_owned()),
        });
    }
    if is_new && params.update_key.is_some() {
        return Ok(WriteResult {
            success: false,
            id: None,
            error: Some(ERR_BOTH.to_owned()),
        });
    }

    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    // DISPATCH-GATING FIX (operator directive 2026-06-17 "honor graph deps in
    // dispatch"): the daemon is the SINGLE writer for `kavach db write` (the CLI
    // routes here), but dispatch readiness (`deps_satisfied`) reads deps ONLY
    // from CONTENT (`parse_declared_deps`). Relationships arrive as graph edges
    // (`params.relationships`) and were written verbatim into content before — so
    // a `--depends-on`-only gate was invisible and the card re-dispatched forever
    // (decision.arch.kavach-depends-on-mirror-wrong-layer: the earlier CLI-side
    // mirror missed THIS path). Mirror `depends_on` edge targets into a
    // `DEPENDS_ON:` content line here, the shared daemon sink, so EVERY write
    // (CLI-RPC + any future RPC caller) gates correctly. Idempotent.
    let content_owned = mirror_depends_on_into_content(
        params.content.as_deref().unwrap_or(""),
        &params.relationships,
    );
    let content = content_owned.as_str();

    let qname = format!("{}/{}/{}", params.project, params.category, params.key);
    let refs: Vec<String> = Vec::new();
    let priority = params.priority.map(Priority::new);
    let result = kavach_surreal::upsert_entry_full()
        .db(&ctx.db)
        .category(&params.category)
        .project_id(&pid)
        .entry_key(&params.key)
        .title(&params.title)
        .content(content)
        .event_source("rpc")
        .qualified_name(&qname)
        .references(&refs)
        .maybe_priority(priority)
        .maybe_exec_prompt(params.exec_prompt.as_deref())
        .build_for_call()
        .await;

    match result {
        Ok(id) => {
            relationships::project_relationships(ctx, &params, &qname).await;
            Ok(WriteResult {
                success: true,
                id: Some(format!("{id:?}")),
                error: None,
            })
        }
        Err(e) => Ok(WriteResult {
            success: false,
            id: None,
            error: Some(e.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::mirror_depends_on_into_content;

    fn dep(target: &str) -> (String, String) {
        ("depends_on".to_owned(), target.to_owned())
    }

    /// A `depends_on` edge is mirrored into a `DEPENDS_ON` content line dispatch reads.
    #[test]
    fn depends_on_edge_is_mirrored_into_content() {
        let out = mirror_depends_on_into_content("body", &[dep("roadmap.unit.x")]);
        assert!(out.starts_with("DEPENDS_ON: roadmap.unit.x"));
        assert!(out.contains("body"));
    }

    /// Non-depends_on relationships (references/blocks) are NOT mirrored.
    #[test]
    fn non_depends_on_edges_are_ignored() {
        let rels = [("references".to_owned(), "roadmap.unit.y".to_owned())];
        assert_eq!(mirror_depends_on_into_content("body", &rels), "body");
    }

    /// Idempotent: a target already on a `DEPENDS_ON` content line is not duplicated.
    #[test]
    fn already_declared_dep_is_not_duplicated() {
        let content = "DEPENDS_ON: roadmap.unit.x\nrest";
        let out = mirror_depends_on_into_content(content, &[dep("roadmap.unit.x")]);
        assert_eq!(out, content);
    }

    /// Empty content + a dep yields exactly the bare `DEPENDS_ON` line.
    #[test]
    fn empty_content_yields_bare_line() {
        assert_eq!(
            mirror_depends_on_into_content("", &[dep("roadmap.unit.x")]),
            "DEPENDS_ON: roadmap.unit.x"
        );
    }

    /// No `depends_on` edges → content returned unchanged.
    #[test]
    fn no_deps_leaves_content_unchanged() {
        assert_eq!(mirror_depends_on_into_content("body", &[]), "body");
    }
}
