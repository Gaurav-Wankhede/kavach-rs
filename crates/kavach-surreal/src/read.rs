// split: intentional - read operations for SurrealDB with query helpers
use crate::dual_write::MemoryEntry;
use crate::error::Result;
use crate::filter::FilterExpr;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;

// `category` is implicit in the table name (decision/research/roadmap/...), so
// SCHEMAFULL doesn't define it. Selecting it would yield NULL on every row,
// which trips serde even with #[serde(default)] (default applies to missing
// keys, not null values). `tags` and `decay_score` aren't defined on every
// per-category table either.
const MEMORY_FIELDS: &str = "id, project, entry_key, title, content, status, entry_status, access_count, created_at, updated_at, priority, lane";

// BUG-FIX [silent-read-drop]: `category` is implicit in the table name and is
// NOT a selected column, so every row deserializes with `category = None` and
// `category_str()` returns "" — printing `[]` in search/query. The read layer
// is the one place that knows which table it queried, so it stamps the category
// back onto each row before returning. Same class as the projects.rs `parent`
// SELECT-omission fix. SOURCE: <https://surrealdb.com/docs/surrealql/statements/select>
fn stamp_category(table: &str, mut entries: Vec<MemoryEntry>) -> Vec<MemoryEntry> {
    for e in &mut entries {
        e.category = Some(table.to_owned());
    }
    entries
}

/// Stamp the implicit category onto a single optional row. Takes `&mut` so it
/// is a statement, not an `Option::map` closure (clippy `single_option_map`).
fn stamp_one(table: &str, entry: &mut Option<MemoryEntry>) {
    if let Some(e) = entry {
        e.category = Some(table.to_owned());
    }
}

// Dispatch ordering: priority ASC (lower = higher rank), NONE last, then
// FIFO by created_at. SurrealDB ORDER BY accepts only identifiers (no
// parenthesised expressions), so the coalesced sort key is projected as
// `priority ?? 999999 AS _sort_priority` and ORDER BY uses the alias.
// SOURCE: https://surrealdb.com/docs/surrealql/clauses/order-by

/// Fetch a memory entry by (project, `entry_key`) from `table`.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn get_by_key(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    key: &str,
) -> Result<Option<MemoryEntry>> {
    let query = format!(
        "SELECT {MEMORY_FIELDS} FROM type::table($table) WHERE project = $project AND entry_key = $key LIMIT 1"
    );
    // SELF-HEALING (read path, increment 2): a transient RocksDB busy/lock spike or
    // mid-session blip is retried with bounded backoff before surfacing. A read is
    // not durability-critical (the caller can re-issue), but the dispatch hot path
    // (next_open_task) reads every tick, so a momentary fault should self-heal
    // rather than fail a scheduler tick. Binds are consumed per `.await`, so the
    // closure rebuilds them each attempt. SOURCE: crate::retry::with_retry.
    let mut entry: Option<MemoryEntry> = crate::retry::with_retry(|| async {
        let mut response = db
            .query(&query)
            .bind(("table", table.to_owned()))
            .bind(("project", project_id.clone()))
            .bind(("key", key.to_owned()))
            .await?;
        response.take(0).map_err(crate::error::Error::Surreal)
    })
    .await?;
    stamp_one(table, &mut entry);
    Ok(entry)
}

/// Fetch a memory entry by its `RecordId` parts (`table`, `id`).
///
/// # Errors
/// Propagates `Error::Surreal` from the typed `select`.
pub async fn get_by_id(db: &Surreal<Db>, table: &str, id: &str) -> Result<Option<MemoryEntry>> {
    let mut entry: Option<MemoryEntry> = db.select((table, id)).await?;
    stamp_one(table, &mut entry);
    Ok(entry)
}

/// List all memory entries in `table` for `project_id`, ordered by priority
/// then `created_at` (dispatch order).
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_by_project(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
) -> Result<Vec<MemoryEntry>> {
    // Compile-time concat! constant — no runtime SQL building. `table` is
    // bound via `$table`, MEMORY_FIELDS + MEMORY_ORDER_BY are static text.
    // Sort makes priority semantics load-bearing: the dispatcher
    // (roadmap::next_open_task) consumes this order directly.
    const QUERY: &str = concat!(
        "SELECT id, project, entry_key, title, content, status, entry_status, ",
        "access_count, created_at, updated_at, priority, lane, ",
        "occupied_by, occupied_until, ",
        "priority ?? 999999 AS _sort_priority ",
        "FROM type::table($table) WHERE project = $project ",
        "ORDER BY _sort_priority ASC, created_at ASC"
    );
    // SELF-HEALING (read path, increment 2): the scheduler consumes this order
    // every dispatch tick, so a transient fault is retried before it fails a tick.
    // See get_by_key for the rationale; binds rebuilt per attempt.
    let entries: Vec<MemoryEntry> = crate::retry::with_retry(|| async {
        let mut response = db
            .query(QUERY)
            .bind(("table", table.to_owned()))
            .bind(("project", project_id.clone()))
            .await?;
        response.take(0).map_err(crate::error::Error::Surreal)
    })
    .await?;
    Ok(stamp_category(table, entries))
}

/// List every row of `table` across ALL projects.
///
/// Dependency keys (`DEPENDS_ON:`/`BLOCKED_BY:`) are a global key space — a
/// card may declare a prerequisite that lives under a different project. A
/// project-scoped lookup cannot see such a row and fail-closes it to
/// "unsatisfied", permanently stalling dispatch. This unscoped read is the
/// resolver's row set so a verified prerequisite counts regardless of project.
///
/// The query is a compile-time `concat!` constant — no runtime SQL string
/// building. The column list is a fixed allowlist and `table` is bound via
/// `$table`, so there is no injection surface.
/// List every row of `table` across ALL projects.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_all_by_table(db: &Surreal<Db>, table: &str) -> Result<Vec<MemoryEntry>> {
    const QUERY: &str = concat!(
        "SELECT id, project, entry_key, title, content, status, entry_status, ",
        "access_count, created_at, updated_at, priority, lane FROM type::table($table)"
    );
    let mut response = db.query(QUERY).bind(("table", table.to_owned())).await?;
    let entries: Vec<MemoryEntry> = response.take(0)?;
    Ok(stamp_category(table, entries))
}

/// List `table` rows for (project, `entry_status`) ordered by priority then
/// `created_at`.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_by_status(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    status: &str,
) -> Result<Vec<MemoryEntry>> {
    // Compile-time concat! constant; same priority-aware ORDER BY as
    // list_by_project so callers (kanban, dispatcher when status-filtered)
    // see the same dispatch order.
    const QUERY: &str = concat!(
        "SELECT id, project, entry_key, title, content, status, entry_status, ",
        "access_count, created_at, updated_at, priority, lane, ",
        "occupied_by, occupied_until, ",
        "priority ?? 999999 AS _sort_priority ",
        "FROM type::table($table) WHERE project = $project AND entry_status = $status ",
        "ORDER BY _sort_priority ASC, created_at ASC"
    );
    let mut response = db
        .query(QUERY)
        .bind(("table", table.to_owned()))
        .bind(("project", project_id.clone()))
        .bind(("status", status.to_owned()))
        .await?;
    let entries: Vec<MemoryEntry> = response.take(0)?;
    Ok(stamp_category(table, entries))
}

/// Query entries with metadata filtering, ordered by `updated_at` desc.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_with_filter(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    filter: Option<&FilterExpr>,
    limit: Option<usize>,
) -> Result<Vec<MemoryEntry>> {
    let mut query =
        format!("SELECT {MEMORY_FIELDS} FROM type::table($table) WHERE project = $project");
    if let Some(f) = filter {
        query.push_str(" AND ");
        query.push_str(&f.to_surql());
    }
    query.push_str(" ORDER BY updated_at DESC");
    if let Some(n) = limit {
        use std::fmt::Write as _;
        // fmt::Write on String is infallible; ignore Ok(()) result.
        let _w = write!(query, " LIMIT {n}");
    }
    let mut response = db
        .query(&query)
        .bind(("table", table.to_owned()))
        .bind(("project", project_id.clone()))
        .await?;
    let entries: Vec<MemoryEntry> = response.take(0)?;
    Ok(stamp_category(table, entries))
}

// REMOVED 2026-05: count_by_status + StatusCounts — counts emitted no
// useful project state. Callers now use `list_by_project` to fetch actual
// roadmap titles + entry_status (real progress, not a tally).
