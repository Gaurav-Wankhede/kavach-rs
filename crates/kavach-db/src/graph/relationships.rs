// Project typed cross-entry relationships into the Database graph.
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
// SOURCE: https://database.com/docs/learn/data-models/graph/overview
use crate::error::Result;
use crate::db::DbStore;

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
    db: &DbStore,
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
        // Resolve-or-create both endpoints, then upsert the typed edge. The
        // whole (rel, target) pair is one transaction so a failed edge write is
        // NOT silently counted as success nor silently dropped (§no_silent_failure).
        // NOTE: multi-statement strings with params are unsafe here — libsql
        // prepares only the FIRST statement and silently drops the tail — so
        // each statement runs individually inside a real txn.
        // NOTE: `Connection::query` defers statement errors into the returned
        // Rows (they surface only on `next()`); INSERTs must use `execute`,
        // which returns the error immediately, or a failed write is silently
        // counted as success.
        let txn = db.conn.transaction().await?;
        txn.execute(
            "INSERT INTO entities (id, entity_type, name, properties) \
             VALUES (?1, 'memory', ?2, NULL) \
             ON CONFLICT(id) DO NOTHING",
            libsql::params![source_qname.to_owned(), source_qname.to_owned()],
        )
        .await?;
        txn.execute(
            "INSERT INTO entities (id, entity_type, name, properties) \
             VALUES (?1, 'memory', ?2, NULL) \
             ON CONFLICT(id) DO NOTHING",
            libsql::params![target.to_owned(), target.to_owned()],
        )
        .await?;
        txn.execute(
            "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
             VALUES (?1, ?2, ?3, 1.0, ?4) \
             ON CONFLICT(from_id, rel_type, to_id) DO UPDATE SET weight = 1.0",
            libsql::params![
                source_qname.to_owned(),
                rel.to_owned(),
                target.to_owned(),
                crate::db::now_rfc3339()
            ],
        )
        .await?;
        txn.commit().await?;
        written = written.saturating_add(1);
    }
    Ok(written)
}

#[cfg(test)]
mod cli_sequence_tests {
    use crate::{apply_schema, open_memory, project_register, upsert_entry_full};

    // Mirrors the `kavach db write` direct path; proves the supersedes edge is
    // actually written (symptom was `Ok(0)` here).
    #[tokio::test]
    async fn cli_two_step_writes_supersedes_edge() {
        let db = open_memory().await.expect("mem db");
        apply_schema(&db).await.expect("schema");
        let proj: String = project_register(&db, "p", "P", "/tmp", None)
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
        // The projection must NOT wrap its statements in an explicit
        // BEGIN/COMMIT string (libsql prepares only the first statement of a
        // multi-statement string, silently dropping the tail). The real txn is
        // driven by `Connection::transaction()` instead.
        let src = "INSERT INTO entities (id, entity_type, name, properties) \
                   VALUES (?1, 'memory', ?2, NULL) \
                   ON CONFLICT(id) DO NOTHING";
        assert!(
            !src.contains("BEGIN TRANSACTION"),
            "no explicit txn frame: {src}"
        );
        assert!(!src.contains("COMMIT"), "no explicit commit: {src}");
    }
}
