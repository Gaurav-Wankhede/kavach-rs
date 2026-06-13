// Recurrence-ranked anti_pattern listing — the read side of the autonomous
// mistake loop. The capture path (append_mistake_event + cluster_event_to_pattern)
// writes anti_pattern centroids into the graph; this query surfaces the top-N by
// recurrence so SessionStart reinjection, the CLI, and the GUI all read the SAME
// store the daemon writes. Closes the read/write split-brain (reinjection used to
// read the legacy `pattern` memory_entries, never these nodes).
// See decision/mistake-loop-close-read-graph.
//
// ALGO: rank a bounded anti_pattern set (one node per behavioral cluster — dozens,
//   not millions) by recurrence count. CHOICE: materialize all rows, then
//   slice::sort_by (Rust stdlib stable sort, Timsort-derived, O(N log N)).
//   REJECTED: SurrealDB `ORDER BY <count-aggregate>` — ordering on a graph-count
//   alias is non-portable across SurrealDB versions; a heap/quickselect partial
//   sort gives no measurable win at N≈dozens and costs clarity.
//   TIME: O(N log N), N = anti_pattern count. SPACE: O(N). YEAR: 2026.
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::SurrealValue;

/// One recurrence-ranked anti-pattern: the clustered behavioral lesson plus how
/// often a `mistake_event` has been routed to it (inbound `instance_of` edges).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AntiPatternRanked {
    /// Canonical node name, e.g. `anti.continuation_menu.395f9852`.
    pub name: String,
    /// Gate that fired the originating mistakes.
    pub gate: String,
    /// The do-instead rule reinjected to reinforce the fix (anti-parrot framing).
    pub correct_action: String,
    /// Recurrence count = inbound `instance_of` edges (the K-PRI signal).
    pub hit_count: i64,
}

/// Top-N anti-patterns ranked by recurrence (descending), then by name for a
/// stable tie-break.
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn top_anti_patterns(db: &Surreal<Db>, limit: usize) -> Result<Vec<AntiPatternRanked>> {
    // gate + correct_action are non-optional: upsert_anti_pattern always sets
    // both. Deserializing as String (not Option) fails closed on a malformed
    // node rather than silently defaulting to "" — the caller then falls back to
    // the legacy ledger instead of reinjecting a blank rule.
    #[derive(SurrealValue)]
    struct Row {
        name: String,
        gate: String,
        correct_action: String,
        hit_count: i64,
    }

    let q = "SELECT name, \
             properties.gate AS gate, \
             properties.correct_action AS correct_action, \
             count(<-instance_of<-entity) AS hit_count \
             FROM entity WHERE entity_type = 'anti_pattern'";
    let mut resp = db.query(q).await?;
    // A brand-new graph has never created the `entity` table (no migration / no
    // prior write), so SELECT raises "table does not exist". That is the empty
    // case — zero anti_patterns — not a failure: return [] so callers render
    // "no mistakes yet" instead of an error.
    let mut rows: Vec<Row> = match resp.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    rows.sort_by(|a, b| {
        b.hit_count
            .cmp(&a.hit_count)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows.truncate(limit);
    Ok(rows
        .into_iter()
        .map(|r| AntiPatternRanked {
            name: r.name,
            gate: r.gate,
            correct_action: r.correct_action,
            hit_count: r.hit_count,
        })
        .collect())
}

#[cfg(test)]
#[path = "top_test.rs"]
mod top_test;
