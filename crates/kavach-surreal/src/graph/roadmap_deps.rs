//! Kanban dependency edges as FIRST-CLASS `SurrealDB` graph edges.
//!
//! Historically the kanban DAG lived only as `DEPENDS_ON:`/`BLOCKED_BY:` TEXT
//! lines in a card's content, parsed in Rust and walked by a hand-rolled DFS
//! (`kavach-rpc roadmap::readiness::cycle`). That bypassed `SurrealDB`'s native
//! graph engine entirely. This module mirrors those declared deps into real
//! `roadmap_card --depends_on--> roadmap_card` RELATE edges so the dependency
//! DAG is queryable with `SurrealDB` graph operators and (3.1+) recursive `{..}`
//! traversal — unifying the kanban DAG with the concept/flow knowledge graph.
//!
//! The card-content `DEPENDS_ON:` line stays the human-authored SOURCE; the
//! RELATE edge is its queryable PROJECTION (the same store-as-graph /
//! author-as-text split the `flow` DAG uses). `mirror_card_deps` is idempotent:
//! it deletes the card's existing out-edges then re-creates them from the
//! current dep list, so re-running on an edited card converges (last-writer
//! wins) without duplicate edges.

use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

use crate::error::{Error, Result};

/// `entity_type` tag for a kanban card node in the shared `entity` graph table.
/// Distinct from `concept`/`flow_step` so a card's edges never collide with the
/// knowledge graph; the unique `idx_entity_type_name` index keys on
/// `(entity_type, name)` so `name = <card key>` is the stable anchor.
const CARD_KIND: &str = "roadmap_card";

/// Validate a card key before it is interpolated as a bound param value. Keys are
/// authored as kanban slugs (alphanumeric + `_`/`-`/`.`); rejecting anything else
/// keeps a hostile/malformed key out of the graph at the edge (illegal states
/// unrepresentable inward). Empty is rejected too.
fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(Error::Migration("roadmap card key cannot be empty".into()));
    }
    if key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Ok(());
    }
    Err(Error::Migration(format!(
        "roadmap card key '{key}' has illegal chars (allowed: alphanumeric _ - .)"
    )))
}

/// Mirror one card's declared dependency keys into `depends_on` RELATE edges.
///
/// Idempotent: clears `card`'s existing out-edges, then RELATEs `card -> dep`
/// for each `dep`. The edge direction is prerequisite-facing the SAME way the
/// scheduler reads it — `card` depends on each `dep`, so `dep` must finish
/// first. Dep keys are NOT required to exist as cards yet (a cross-project or
/// not-yet-created prereq); the anchor is `UPSERT`ed so the edge is never lost,
/// matching the fail-safe "unknown dep blocks" tolerance of the text path.
///
/// # Errors
/// Propagates `Error::Migration` on an invalid key and `Error::Surreal` on any
/// query failure. Runs in one transaction so a mid-op crash leaves no partial
/// edge set (failure-lens: no orphaned half-mirror).
pub async fn mirror_card_deps(db: &Surreal<Db>, card_key: &str, deps: &[String]) -> Result<()> {
    validate_key(card_key)?;
    for d in deps {
        validate_key(d)?;
    }
    // Build the RELATE clauses with positional bound params (no key
    // interpolation into the query text — keys are validated AND bound). Each dep
    // gets its own bound param dep0, dep1, … . Built by mapping the index range to
    // owned clause strings and concatenating — no `write!` (whose infallible-on-
    // String Result would force a silent discard) and no `format!`-into-`push_str`
    // (which clippy::format_push_string flags); a plain iterator + concat is both
    // discard-free and lint-clean.
    // Map each index to an owned clause, collect into a Vec, then concat to one
    // String. No Result anywhere (so nothing to discard or expect), no
    // push_str(&format!) and no format!-in-collect — the one construction that
    // satisfies the full strict-lint set (expect_used / format_push_string /
    // format_collect / dropping_copy all deny the alternatives).
    let dep_clauses: String = (0..deps.len())
        .map(|i| {
            format!(
                "LET $d{i} = (UPSERT entity SET entity_type = $kind, name = $dep{i}, \
                     updated_at = time::now() WHERE entity_type = $kind AND name = $dep{i} \
                     RETURN id)[0].id; \
                 RELATE $src->depends_on->$d{i} SET mirrored_at = time::now(); "
            )
        })
        .collect::<Vec<String>>()
        .concat();
    let q = format!(
        "BEGIN TRANSACTION; \
         LET $src = (UPSERT entity SET entity_type = $kind, name = $card, \
             updated_at = time::now() WHERE entity_type = $kind AND name = $card \
             RETURN id)[0].id; \
         DELETE $src->depends_on; \
         {dep_clauses}COMMIT TRANSACTION;"
    );

    let mut query = db
        .query(q)
        .bind(("kind", CARD_KIND))
        .bind(("card", card_key.to_owned()));
    for (i, d) in deps.iter().enumerate() {
        query = query.bind((format!("dep{i}"), d.clone()));
    }
    query.await?.check()?;
    Ok(())
}

/// SQL-native cycle check: does `card_key` participate in a `depends_on` cycle?
///
/// Uses `SurrealDB` 3.1 recursive graph traversal `->depends_on->{..}` to walk the
/// transitive prerequisite closure, then asks whether the start node is reachable
/// from itself. This is the graph-engine equivalent of the Rust three-color DFS
/// in `kavach-rpc roadmap::readiness::cycle::is_in_cycle` — including the
/// self-dependency boundary case (`A` depends on `A`), which appears as `A` in
/// its own recursive closure. Returns `true` iff a cycle reaches `card_key`.
///
/// # Errors
/// Propagates `Error::Migration` on an invalid key, `Error::Surreal` on query
/// failure.
pub async fn is_in_cycle_sql(db: &Surreal<Db>, card_key: &str) -> Result<bool> {
    validate_key(card_key)?;
    // The recursive closure `->depends_on->{..}` yields every node transitively
    // reachable via prerequisite edges. If the start card's own name appears in
    // that closure, a back-edge closed onto it => cycle. `array::any` over a
    // name match keeps the decision in the DB (single round-trip).
    // `SurrealDB` 3.1 recursive path traversal: `$start.{..+collect}->depends_on->entity`
    // walks the prerequisite closure to unbounded depth and returns the unique set
    // of reachable records (the `+collect` algorithm). We map to their names and
    // test membership of the start card's own name — its presence proves a
    // back-edge closed onto the start => cycle. `SurrealDB`'s recursion does not
    // revisit a node, so a self/mutual dependency terminates without infinite loop.
    let q = "LET $start = (SELECT id FROM entity \
                 WHERE entity_type = $kind AND name = $card)[0].id; \
             RETURN IF $start = NONE THEN false ELSE \
                 ($start.{..+collect}->depends_on->entity).name CONTAINS $card END;";
    let mut res = db
        .query(q)
        .bind(("kind", CARD_KIND))
        .bind(("card", card_key.to_owned()))
        .await?
        .check()?;
    let cyclic: Option<bool> = res.take(1)?;
    Ok(cyclic.unwrap_or(false))
}

#[cfg(test)]
#[path = "roadmap_deps_test.rs"]
#[cfg(test)]
#[path = "roadmap_deps_test.rs"]
mod tests;