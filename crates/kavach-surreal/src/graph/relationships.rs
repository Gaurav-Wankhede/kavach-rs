// Project typed cross-entry relationships into the SurrealDB graph.
//
// [RCA]
// symptom:    The entity graph has 4235 nodes but ~0 semantic edges between them.
// repro:      kavach db graph-query --relationship depends_on returns no entries.
// why5:       upsert_entry_full only creates entity->in_scope->project plus
//             references->skill from wikilinks. Cross-entry edges (roadmap
//             depends_on roadmap, decision supersedes decision, etc.) are never
//             produced because no extractor reads frontmatter directives.
// root_cause: Missing typed-relationship projection pass at write time.
// fix:        This module. CLI write path calls upsert_relationships(...) after
//             upsert_entry_full so each frontmatter directive yields one edge.
//
// [SDUI_DECISION]
// protocol: in-process Rust call
// placement: rels: &[(rel, target)] as fn arg
// pagination: none
// versioning: component-level (additive)
// envelope: no
// caching: none
// failure_modes: unknown rel -> skip; empty qname -> skip
// [/SDUI_DECISION]
//
// ALGO: SequentialUpsertRelate
// PROBLEM_CLASS: graph
// REJECTED: [{"name":"single_RELATE_no_upsert","reason":"target may not exist yet"},{"name":"hash_lookup_existing","reason":"UPSERT idempotency cheaper"}]
// TIME: O(r) | SPACE: O(r)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: each rel is its own round-trip
// BENCHMARK: https://surrealdb.com/docs/surrealql/statements/relate
// SOURCE: https://surrealdb.com/docs/learn/data-models/graph/overview
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

/// Allowed inter-entry relationship types. Subset of `graph::dynamic::ALLOWED_RELS`
/// that makes semantic sense between memory rows (not skill/algorithm/project).
const ALLOWED_INTER_ENTRY_RELS: &[&str] = &[
    "depends_on",
    "blocks",
    "supersedes",
    "references",
    "mentions",
];

/// Upsert RELATE edges from `source_qname` (must already be an entity) to each
/// `(rel, target_qname)`. Targets auto-created as memory entities.
///
/// # Errors
///
/// Propagates `Result::Err` when the query transaction fails.
pub async fn upsert_relationships(
    db: &Surreal<Db>,
    source_qname: &str,
    rels: &[(String, String)],
) -> Result<usize> {
    if source_qname.is_empty() || rels.is_empty() {
        return Ok(0);
    }
    let mut written = 0usize;
    for (rel, target) in rels {
        if !ALLOWED_INTER_ENTRY_RELS.contains(&rel.as_str()) {
            continue;
        }
        if target.is_empty() {
            continue;
        }
        let q = format!(
            "BEGIN TRANSACTION; \
             LET $src = (UPSERT entity SET entity_type = 'memory', name = $src_name, \
                 updated_at = time::now() WHERE entity_type = 'memory' AND name = $src_name \
                 RETURN id)[0].id; \
             LET $tgt = (UPSERT entity SET entity_type = 'memory', name = $tgt_name, \
                 updated_at = time::now() WHERE entity_type = 'memory' AND name = $tgt_name \
                 RETURN id)[0].id; \
             RELATE $src->{rel}->$tgt SET weight = 1.0; \
             COMMIT TRANSACTION;"
        );
        let resp = db
            .query(q)
            .bind(("src_name", source_qname.to_owned()))
            .bind(("tgt_name", target.to_owned()))
            .await?;
        if resp.check().is_ok() {
            written = written.saturating_add(1);
        }
    }
    Ok(written)
}
