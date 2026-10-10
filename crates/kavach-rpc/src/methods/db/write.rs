// TIME: O(1) avg | SPACE: O(1)
//! db.write RPC method — create or update entry.

use super::util::resolve_project_id;
use crate::methods::roadmap::readiness::bare_dep_key;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_types::Priority;
use serde::{Deserialize, Serialize};

mod exec_prompt_gate;
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
///
/// Idempotent by BARE KEY, not by literal token. A previous version compared
/// tokens literally, so a content line already carrying `kavach-rs/roadmap/foo`
/// never matched an incoming bare `foo` — and the mirror appended a SECOND
/// `DEPENDS_ON:` line. 52 roadmap rows carry exactly that pair. Dep targets
/// reach this function in both forms (the CLI qualifies flag targets; the NLU
/// extractor emits bare keys), so the comparison must be form-insensitive.
fn mirror_depends_on_into_content(content: &str, relationships: &[(String, String)]) -> String {
    // Dedupe within the batch too: two edges naming the same dep by different
    // forms must yield one line entry.
    let mut fresh: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (_, target) in relationships.iter().filter(|(rel, _)| rel == "depends_on") {
        let t = target.trim();
        if t.is_empty() {
            continue;
        }
        let bare = bare_dep_key(t).to_owned();
        if content_declares_dep(content, &bare) || seen.contains(&bare) {
            continue;
        }
        seen.push(bare.clone());
        fresh.push(bare);
    }
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

/// `true` iff `content` already names `dep` on a `DEPENDS_ON:` line
/// — the same lines the readiness parser reads. Whitespace/comma tolerant, and
/// BARE-TAIL tolerant so a project-qualified token on the line matches a bare
/// incoming key (and vice versa).
fn content_declares_dep(content: &str, dep: &str) -> bool {
    let want = bare_dep_key(dep);
    content
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("DEPENDS_ON:"))
        .any(|l| {
            l.split([':', ',', ' ', '\t'])
                .map(str::trim)
                .filter(|tok| !tok.is_empty())
                .any(|tok| bare_dep_key(tok) == want)
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

    if let Some(reason) =
        exec_prompt_gate::blocked(&params.category, is_new, params.exec_prompt.as_deref())
    {
        return Ok(WriteResult {
            success: false,
            id: None,
            error: Some(reason.to_owned()),
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
    let result = kavach_db::upsert_entry_full()
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

    /// REGRESSION: a content line already carrying the PROJECT-QUALIFIED
    /// spelling of a dep must block the incoming BARE spelling from appending a
    /// second line. Both mirrors run here (the CLI qualifies flag targets, the
    /// NLU extractor emits bare keys) and the literal token compare let both
    /// spellings through — 52 roadmap rows now carry the pair.
    #[test]
    fn qualified_dep_on_line_blocks_the_bare_spelling() {
        let content = "DEPENDS_ON: kavach-rs/roadmap/roadmap.unit.x\nrest";
        let out = mirror_depends_on_into_content(content, &[dep("roadmap.unit.x")]);
        assert_eq!(
            out, content,
            "bare key must match the qualified token already on the line"
        );
        assert_eq!(
            out.lines().filter(|l| l.starts_with("DEPENDS_ON:")).count(),
            1,
            "exactly one dep line"
        );
    }

    /// The mirror normalises to the BARE tail, so the line it writes is the same
    /// spelling the readiness parser and `db doctor repair-deps` agree on.
    #[test]
    fn mirrored_line_uses_the_bare_tail() {
        let out = mirror_depends_on_into_content("body", &[dep("kavach-rs/roadmap/unit.x")]);
        assert!(out.starts_with("DEPENDS_ON: unit.x"), "got: {out}");
    }

    /// Two edges naming the same dep by different forms yield ONE line entry.
    #[test]
    fn same_dep_in_two_forms_yields_one_entry() {
        let rels = [dep("kavach-rs/roadmap/unit.x"), dep("unit.x")];
        let out = mirror_depends_on_into_content("body", &rels);
        assert_eq!(
            out.lines().filter(|l| l.starts_with("DEPENDS_ON:")).count(),
            1
        );
        assert_eq!(out.matches("unit.x").count(), 1, "got: {out}");
    }

    /// A genuinely new dep alongside an already-declared one is still added, and
    /// the existing one is not repeated.
    #[test]
    fn a_new_dep_is_added_alongside_an_existing_one() {
        let content = "DEPENDS_ON: kavach-rs/roadmap/unit.x\nrest";
        let out = mirror_depends_on_into_content(content, &[dep("unit.y")]);
        assert_eq!(
            out, "DEPENDS_ON: unit.y\nDEPENDS_ON: kavach-rs/roadmap/unit.x\nrest",
            "new dep prepended, existing preserved verbatim"
        );
    }

    #[test]
    fn empty_target_is_ignored() {
        assert_eq!(mirror_depends_on_into_content("body", &[dep("  ")]), "body");
    }
}
