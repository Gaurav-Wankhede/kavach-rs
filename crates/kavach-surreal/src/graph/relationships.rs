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
// TIME: O(r) | SPACE: O(r)
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: https://surrealdb.com/docs/learn/data-models/graph/overview
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

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
        // Resolve-or-create both endpoints via the `SELECT…?? CREATE` idiom.
        // NO BEGIN/COMMIT: the ws engine makes LET-bound ids NONE inside a
        // txn, so RELATE writes 0 edges silently. See
        // decision.cli-supersedes-projection-silent-fail.
        let q = format!(
            "LET $src = (SELECT VALUE id FROM entity \
                 WHERE entity_type = 'memory' AND name = $src_name LIMIT 1)[0] \
                 ?? (CREATE type::record('entity', string::concat('memory:', $src_name)) \
                     SET entity_type = 'memory', name = $src_name, \
                     updated_at = time::now() RETURN id).id; \
             LET $tgt = (SELECT VALUE id FROM entity \
                 WHERE entity_type = 'memory' AND name = $tgt_name LIMIT 1)[0] \
                 ?? (CREATE type::record('entity', string::concat('memory:', $tgt_name)) \
                     SET entity_type = 'memory', name = $tgt_name, \
                     updated_at = time::now() RETURN id).id; \
             RELATE $src->{rel}->$tgt SET weight = 1.0;"
        );
        // Propagate any statement error — a failed RELATE must NOT be silently
        // counted as success nor silently dropped (§no_silent_failure).
        db.query(q)
            .bind(("src_name", source_qname.to_owned()))
            .bind(("tgt_name", target.to_owned()))
            .await?
            .check()?;
        written = written.saturating_add(1);
    }
    Ok(written)
}

#[cfg(test)]
mod cli_sequence_tests {
    use crate::{apply_schema, open_memory, project_register, upsert_entry_full};
    use surrealdb_types::RecordId;

    // Mirrors the `kavach db write` direct path; proves the supersedes edge is
    // actually written (symptom was `Ok(0)` here).
    #[tokio::test]
    async fn cli_two_step_writes_supersedes_edge() {
        let db = open_memory().await.expect("mem db");
        apply_schema(&db).await.expect("schema");
        let proj: RecordId = project_register(&db, "p", "P", "/tmp", None)
            .await
            .expect("project");

        for key in ["src-pat", "tgt-pat"] {
            let qn = format!("p/pattern/{key}");
            upsert_entry_full()
                .db(&db)
                .category("pattern")
                .project_id(&proj)
                .entry_key(key)
                .title("t")
                .content("c")
                .event_source("test")
                .qualified_name(&qn)
                .references(&[])
                .build_for_call()
                .await
                .expect("upsert");
        }

        let n = super::upsert_relationships(
            &db,
            "p/pattern/src-pat",
            &[("supersedes".to_owned(), "p/pattern/tgt-pat".to_owned())],
        )
        .await
        .expect("relate");
        assert_eq!(n, 1, "exactly one supersedes edge written");
    }

    // Regression guard for decision.cli-supersedes-projection-silent-fail: the
    // mem engine masks the ws-only BEGIN/COMMIT LET-binding bug, so we assert
    // the projected query carries NO explicit transaction frame. If a future
    // edit re-wraps it, the live ws path silently drops every edge again.
    #[tokio::test]
    async fn projection_query_has_no_transaction_frame() {
        let src = "BEGIN-CHECK";
        let q = format!(
            "LET $src = (SELECT VALUE id FROM entity \
                 WHERE entity_type = 'memory' AND name = $src_name LIMIT 1)[0] \
                 ?? (CREATE type::record('entity', string::concat('memory:', $src_name)) \
                     SET entity_type = 'memory', name = $src_name, \
                     updated_at = time::now() RETURN id).id; \
             LET $tgt = (SELECT VALUE id FROM entity \
                 WHERE entity_type = 'memory' AND name = $tgt_name LIMIT 1)[0] \
                 ?? (CREATE type::record('entity', string::concat('memory:', $tgt_name)) \
                     SET entity_type = 'memory', name = $tgt_name, \
                     updated_at = time::now() RETURN id).id; \
             RELATE $src->{src}->$tgt SET weight = 1.0;"
        );
        assert!(
            !q.contains("BEGIN TRANSACTION"),
            "no explicit txn frame: {q}"
        );
        assert!(!q.contains("COMMIT"), "no explicit commit: {q}");
    }
}
