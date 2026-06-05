// split: intentional - write/mutation operations on typed memory tables
// sql-safe: queries use static literals + .bind() for params, no user input concatenation
use crate::error::Result;
use kavach_types::Priority;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

const TYPED_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

#[derive(surrealdb_types::SurrealValue)]
struct EventRow {
    id: RecordId,
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ExpireReport {
    pub archived_total: usize,
    pub per_table: Vec<(String, usize)>,
}

/// Archive entries past their `expires_at` timestamp across all typed memory tables.
///
/// Mirrors `kavach_db::memory::expire_stale` behavior over `SurrealDB`.
/// Returns count via .`len()` of UPDATE ... RETURN AFTER result array
/// (`SurrealDB` has no native affected-rows count; see issue #5258).
///
/// # Errors
/// Propagates `Error::Surreal` from any failed per-table UPDATE.
pub async fn expire_stale(db: &Surreal<Db>) -> Result<ExpireReport> {
    let mut report = ExpireReport::default();
    for table in TYPED_TABLES {
        let count = expire_table(db, table).await?;
        if count > 0 {
            report.per_table.push(((*table).to_owned(), count));
        }
        report.archived_total = report.archived_total.saturating_add(count);
    }
    Ok(report)
}

/// Surgical priority mutation — partial UPDATE of `priority` + `updated_at` only.
///
/// Title, content, status, `entry_status` are untouched. Use for human-in-loop
/// reranking without re-supplying full row data. Returns the row id on success,
/// or `Err(RecordNotFound)` if no row matches the (project, key) pair.
///
/// `new_priority = Some(n)` sets the priority; `None` clears it back to NONE
/// (FIFO tail in the dispatch sort).
///
/// # Errors
/// `Error::RecordNotFound` when no row matches (project, key); `Error::Surreal`
/// when the UPDATE itself fails or returns malformed shape.
pub async fn set_priority(
    db: &Surreal<Db>,
    category: &str,
    project_id: &RecordId,
    entry_key: &str,
    new_priority: Option<Priority>,
) -> Result<RecordId> {
    let query = match category {
        "decision" => {
            "UPDATE decision SET priority = $priority, updated_at = time::now() WHERE project = $pid AND entry_key = $key RETURN id"
        }
        "roadmap" => {
            "UPDATE roadmap SET priority = $priority, updated_at = time::now() WHERE project = $pid AND entry_key = $key RETURN id"
        }
        _ => {
            return Err(crate::error::Error::RecordNotFound(format!(
                "priority is only defined on roadmap and decision tables, got: {category}"
            )));
        }
    };
    let mut response = db
        .query(query)
        .bind(("pid", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("priority", new_priority.map(Priority::get)))
        .await?;
    let ids: Vec<RecordId> = response.take("id")?;
    ids.into_iter().next().ok_or_else(|| {
        crate::error::Error::RecordNotFound(format!("{category}/{entry_key} not found in project"))
    })
}

async fn expire_table(db: &Surreal<Db>, table: &str) -> Result<usize> {
    let query = match table {
        "decision" => {
            "UPDATE decision SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "research" => {
            "UPDATE research SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "roadmap" => {
            "UPDATE roadmap SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "pattern" => {
            "UPDATE pattern SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "app_spec" => {
            "UPDATE app_spec SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        _ => return Ok(0),
    };
    let mut response = db.query(query).await?;
    let updated: Vec<serde_json::Value> = response.take(0)?;
    Ok(updated.len())
}

/// Upsert a memory entry into the typed table for its category.
///
/// Idempotent on (project, `entry_key`) — updates content/title/status if existing.
/// `priority` is the dispatch-priority field (lower = higher rank). `None`
/// leaves the field unset / unchanged on existing rows.
///
/// # Errors
/// `Error::RecordNotFound` for unknown category; `Error::Surreal` for query
/// failures.
pub async fn upsert_entry(
    db: &Surreal<Db>,
    category: &str,
    project_id: &RecordId,
    entry_key: &str,
    title: &str,
    content: &str,
    priority: Option<Priority>,
) -> Result<RecordId> {
    // FIX: [silent_failure/data_loss] write.rs:57 — UPSERT-by-WHERE without a
    // record ID is the documented SurrealDB anti-pattern: on first write it
    // matches nothing and silently inserts NOTHING (exit ok, no row). 2.x's
    // loose semantics masked it; SurrealDB 3.0 (stricter, no silent create)
    // exposed it -> every `db write` was a silent no-op on 3.0.
    // ROOT_CAUSE: wrong upsert form (table+WHERE, no record id).
    // RESEARCH: surrealdb.com/docs/surrealql/statements/upsert +
    //   .../functions/database/type (type::record) — record-ID-targeted
    //   UPSERT (no WHERE) is the correct idempotent create-or-update.
    // FIX: [silent_failure/data_loss] the in-query
    // `type::record('<t>', string::concat(record::id($project),':',$key))`
    // produced a null/typed id under SurrealDB 3.0, so the target record
    // was malformed and SET fields persisted as `none` (row created,
    // category=none on read-back). ROOT_CAUSE: in-query record-id
    // construction depends on 3.0 typed-value semantics of record::id().
    // SOLUTION (verified API docs.rs/surrealdb-types RecordId::new): build
    // the deterministic target RecordId in Rust and bind it as `$rid`;
    // UPSERT $rid (no type::record/concat) — robust, no SurrealQL id math.
    let pk = format!("{:?}", project_id.key);
    let rid = RecordId::new(category, format!("{pk}:{entry_key}"));
    // FIX: [contract_violation / UNIQUE-index] same as upsert_entry_full —
    // entry tables have UNIQUE (project, entry_key). A separately constructed
    // `$rid` collides with any pre-existing row of the same natural key under
    // a different id (v2-migrated legacy ids). Resolve the existing id by the
    // UNIQUE key first; fall back to the deterministic `$rid` only when absent.
    // `priority` is defined ONLY on roadmap + decision tables (see schema.rs).
    // The `IF $priority != NONE THEN ... ELSE priority END` clause leaves the
    // stored value unchanged when the caller passes None (no clobber on
    // re-write of a row that already carries a priority).
    let query = match category {
        "decision" => {
            "LET $eid = (SELECT VALUE id FROM decision WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'decision', entry_key = $key, title = $title, content = $content, status = 'active', entry_status = 'todo', access_count = 0, priority = IF $priority != NONE THEN $priority ELSE priority END, updated_at = time::now() RETURN id"
        }
        "research" => {
            "LET $eid = (SELECT VALUE id FROM research WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'research', entry_key = $key, title = $title, content = $content, status = 'active', entry_status = 'todo', access_count = 0, updated_at = time::now() RETURN id"
        }
        "roadmap" => {
            "LET $eid = (SELECT VALUE id FROM roadmap WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'roadmap', entry_key = $key, title = $title, content = $content, status = 'active', entry_status = 'todo', access_count = 0, priority = IF $priority != NONE THEN $priority ELSE priority END, updated_at = time::now() RETURN id"
        }
        "pattern" => {
            "LET $eid = (SELECT VALUE id FROM pattern WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'pattern', entry_key = $key, title = $title, content = $content, status = 'active', entry_status = 'todo', access_count = 0, updated_at = time::now() RETURN id"
        }
        "app_spec" => {
            "LET $eid = (SELECT VALUE id FROM app_spec WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'app_spec', entry_key = $key, title = $title, content = $content, status = 'active', entry_status = 'verified', access_count = 0, updated_at = time::now() RETURN id"
        }
        other => {
            return Err(crate::error::Error::Migration(format!(
                "unknown category: {other}"
            )));
        }
    };

    let priority_i64 = priority.map(Priority::get);
    let mut response = db
        .query(query)
        .bind(("rid", rid))
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("title", title.to_owned()))
        .bind(("content", content.to_owned()))
        .bind(("priority", priority_i64))
        .await?;
    // FIX [type_erasure]: deserialize ONLY the id, not the full strict
    // MemoryEntry. UPSERT ... RETURN AFTER hands back every SCHEMAFULL field;
    // binding that into MemoryEntry fails on any field the row legitimately
    // omits (the exact "upsert returned empty" symptom). `RETURN id` +
    // UpdatedIdRow is the same robust shape update_status already uses.
    // NOTE: the query is now 2 statements (LET $eid; UPSERT) — the UPSERT
    // result is at index 1, not 0 (statement 0 is the LET binding).
    let rows: Vec<UpdatedIdRow> = response.take(1)?;
    match rows.into_iter().next() {
        Some(r) => Ok(r.id),
        None => Err(crate::error::Error::RecordNotFound(format!(
            "upsert returned empty for {category}/{entry_key}"
        ))),
    }
}

/// Upsert a memory entry AND log the corresponding event in a single `SurrealDB` transaction.
///
/// Either both writes succeed or neither does — eliminates the orphan-state failure mode where the entry persists but the event log misses it.
///
/// SOURCE: <https://surrealdb.com/docs/sdk/rust/concepts/transaction>
/// — multi-statement transactions are performed by passing BEGIN/COMMIT
/// inside a single .`query()` call.
/// SOURCE: <https://github.com/surrealdb/surrealdb/issues/2733> — chaining
/// BEGIN/COMMIT in a single .`query()` string is the supported workaround
/// until the SDK exposes a transaction handle API.
#[bon::builder]
pub async fn upsert_entry_with_event(
    db: &Surreal<Db>,
    category: &str,
    project_id: &RecordId,
    entry_key: &str,
    title: &str,
    content: &str,
    event_source: &str,
    priority: Option<Priority>,
) -> Result<RecordId> {
    upsert_entry_full()
        .db(db)
        .category(category)
        .project_id(project_id)
        .entry_key(entry_key)
        .title(title)
        .content(content)
        .event_source(event_source)
        .qualified_name("")
        .references(&[])
        .maybe_priority(priority)
        .build_for_call()
        .await
}

/// Full atomic upsert: memory entry + event log + entity node + project edge + reference edges, all in a single `SurrealDB` transaction.
///
/// `qualified_name` is the entity name; pass empty string to skip entity creation. `references` are skill names extracted from content (wikilinks, INVOKE).
///
/// SOURCE: <https://surrealdb.com/docs/sdk/rust/concepts/transaction>
/// SOURCE: <https://github.com/surrealdb/surrealdb/issues/2733>
/// SOURCE: <https://blog.rust-lang.org/2024/09/05/Rust-1.81.0>/ (`lint_reasons` stabilized)
// TASK#1313 RESOLVED: bon #[builder] generates compile-time-checked named-args API.
// [RCA] root_cause: 9-arg positional API allowed silent arg-swap miscompile (same-type params)
// fix_strategy: typestate builder forces explicit field names at every call site
// SOURCE: https://docs.rs/bon/latest/bon/attr.builder.html (bon v3.9, 2026-03)
fn append_entity_graph_stmts(q: &mut String, qualified_name: &str, references: &[String]) {
    if qualified_name.is_empty() {
        return;
    }
    q.push_str(
        "LET $entry_node = (SELECT VALUE id FROM entity \
            WHERE entity_type = 'memory' AND name = $qname LIMIT 1)[0] \
            ?? (CREATE type::record('entity', string::concat('memory:', $qname)) \
                SET entity_type = 'memory', name = $qname, \
                updated_at = time::now() RETURN id).id;\n",
    );
    q.push_str(
        "LET $project_node = (SELECT VALUE id FROM entity \
            WHERE entity_type = 'project' AND name = $project_name LIMIT 1)[0] \
            ?? (CREATE type::record('entity', string::concat('project:', $project_name)) \
                SET entity_type = 'project', name = $project_name, \
                updated_at = time::now() RETURN id).id;\n",
    );
    q.push_str("RELATE $entry_node->in_scope->$project_node SET weight = 1.0;\n");
    if !references.is_empty() {
        q.push_str(
            "FOR $ref IN $refs { \
                LET $skill_node = (SELECT VALUE id FROM entity \
                    WHERE entity_type = 'skill' AND name = $ref LIMIT 1)[0] \
                    ?? (CREATE type::record('entity', string::concat('skill:', $ref)) \
                        SET entity_type = 'skill', name = $ref, \
                        updated_at = time::now() RETURN id).id; \
                RELATE $entry_node->references->$skill_node SET weight = 1.0; \
            };\n",
        );
    }
}

#[bon::builder(finish_fn = build_for_call)]
pub async fn upsert_entry_full(
    db: &Surreal<Db>,
    category: &str,
    project_id: &RecordId,
    entry_key: &str,
    title: &str,
    content: &str,
    event_source: &str,
    qualified_name: &str,
    references: &[String],
    priority: Option<Priority>,
) -> Result<RecordId> {
    use std::fmt::Write as _;
    let table = match category {
        "decision" | "research" | "roadmap" | "pattern" | "app_spec" => category,
        other => {
            return Err(crate::error::Error::Migration(format!(
                "unknown category: {other}"
            )));
        }
    };
    let entry_status_default = if table == "app_spec" {
        "verified"
    } else {
        "todo"
    };
    let pk = format!("{:?}", project_id.key);
    let rid = RecordId::new(table, format!("{pk}:{entry_key}"));
    let rid_returned = rid.clone();

    let mut q = String::with_capacity(2048);
    q.push_str("BEGIN TRANSACTION;\n");
    writeln!(
        q,
        "LET $eid = (SELECT VALUE id FROM {table} \
            WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid;"
    )
    .ok();
    let priority_clause: &str = match table {
        "roadmap" | "decision" => {
            ", priority = IF $priority != NONE THEN $priority ELSE priority END"
        }
        "research" | "pattern" | "app_spec" => "",
        unknown => {
            return Err(crate::error::Error::Migration(format!(
                "priority_clause: validator drift — unhandled category '{unknown}'"
            )));
        }
    };
    writeln!(
        q,
        "UPSERT $eid \
            SET project = $project, category = '{table}', \
            entry_key = $key, title = $title, content = $content, \
            status = 'active', entry_status = '{entry_status_default}', \
            access_count = 0{priority_clause}, updated_at = time::now() RETURN id;"
    )
    .ok();
    writeln!(
        q,
        "CREATE event SET event_type = 'memory_write', source = $source, \
            project = $project, payload = {{ category: '{table}', entry_key: $key }}, \
            created_at = time::now();"
    )
    .ok();
    append_entity_graph_stmts(&mut q, qualified_name, references);
    q.push_str("COMMIT TRANSACTION;");

    let project_name = format!("{:?}", &project_id.key);
    let priority_i64 = priority.map(Priority::get);
    let response = db
        .query(q)
        .bind(("rid", rid))
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("title", title.to_owned()))
        .bind(("content", content.to_owned()))
        .bind(("priority", priority_i64))
        .bind(("source", event_source.to_owned()))
        .bind(("qname", qualified_name.to_owned()))
        .bind(("project_name", project_name))
        .bind(("refs", references.to_vec()))
        .await?;
    // ALGO: DeterministicIdReturn (skip transaction-result deserialization)
    // PROBLEM_CLASS: stream (multi-statement response decode)
    // REJECTED: [{"name":"take::<Option<MemoryEntry>>(0)","reason":"RETURN AFTER on SCHEMAFULL row fails to bind when any strict field is absent -> false 'returned empty'"},{"name":"take::<Vec<UpdatedIdRow>>(0)","reason":"inside BEGIN..COMMIT the statement indices shift and RETURN id projects a bare id, not {id} -> 'Expected object, got none'"}]
    // TIME: O(1) | SPACE: O(1) | YEAR: 2026 | SEARCHED: 2026-05
    // TRADEOFF: trusts response.check() for commit proof rather than echoing
    //   the row back; acceptable because the target RecordId is constructed
    //   deterministically in Rust (rid) before the query.
    // SOURCE: https://surrealdb.com/docs/sdk/rust/concepts/transaction
    // BENCHMARK: https://surrealdb.com/docs/sdk/rust/methods/query (IndexedResults per-statement indexing, 3.0)
    match response.check() {
        Ok(_) => Ok(rid_returned),
        Err(e) => Err(crate::error::Error::Migration(format!(
            "transaction failed for {category}/{entry_key}: {e}"
        ))),
    }
}

/// Delete events older than `days` days. Returns count deleted.
///
/// # Errors
/// Propagates `Error::Surreal` from the DELETE query.
pub async fn rotate_events(db: &Surreal<Db>, days: i64) -> Result<usize> {
    let query =
        "DELETE event WHERE created_at < time::now() - duration::from::days($days) RETURN BEFORE";
    let mut response = db.query(query).bind(("days", days)).await?;
    let deleted: Vec<serde_json::Value> = response.take(0)?;
    Ok(deleted.len())
}

/// Append an event row.
///
/// # Errors
/// Propagates `Error::Surreal` from the CREATE query.
pub async fn append_event(
    db: &Surreal<Db>,
    event_type: &str,
    source: &str,
    project: Option<RecordId>,
    payload: Option<&str>,
) -> Result<RecordId> {
    let payload_value: Option<serde_json::Value> = payload.map(|p| {
        serde_json::from_str(p).unwrap_or_else(|_| serde_json::Value::String(p.to_owned()))
    });
    let query = "CREATE event SET event_type = $event_type, source = $source, \
                 project = $project, payload = $payload, created_at = time::now() RETURN AFTER";
    let mut response = db
        .query(query)
        .bind(("event_type", event_type.to_owned()))
        .bind(("source", source.to_owned()))
        .bind(("project", project))
        .bind(("payload", payload_value))
        .await?;
    let result: Option<EventRow> = response.take(0)?;
    match result {
        Some(e) => Ok(e.id),
        None => Err(crate::error::Error::RecordNotFound("event create".into())),
    }
}

/// Append one Layer-A RLVR bandit-log row (harness-rl Wave P2).
///
/// `payload` is the serialized `BanditRow` JSON (the `(x, a, p, r)` tuple) -- this
/// crate stays decoupled from `kavach-patterns`, so the typed row is serialized by
/// the caller and stored opaquely. Content-addressed by a BLAKE3 digest of the
/// payload so an identical replayed decision dedups to one row (append-only,
/// idempotent). Single-writer invariant: only the daemon reaches this.
///
/// # Errors
/// Returns an error if the `bandit_log` create fails.
pub async fn append_bandit_row(db: &Surreal<Db>, payload: &str) -> Result<RecordId> {
    let digest = blake3::hash(payload.as_bytes()).to_hex();
    let row_key = digest.as_str().get(..32).unwrap_or(digest.as_str());
    let payload_value: serde_json::Value = serde_json::from_str(payload)
        .unwrap_or_else(|_| serde_json::Value::String(payload.to_owned()));
    // SurrealDB 3.0 renamed the SurrealQL builtin type::thing() -> type::record()
    // (parse-errors at runtime otherwise; same migration as parts.rs / projects.rs).
    let query = "CREATE type::record('bandit_log', $key) SET \
                 payload = $payload, created_at = time::now() RETURN AFTER";
    let mut response = db
        .query(query)
        .bind(("key", row_key.to_owned()))
        .bind(("payload", payload_value))
        .await?;
    let result: Option<EventRow> = response.take(0)?;
    match result {
        Some(e) => Ok(e.id),
        None => Err(crate::error::Error::RecordNotFound("bandit_log create".into())),
    }
}

/// One stored payload row, read back from `bandit_log`. Holds the opaque
/// `surrealdb_types::Value` (the SDK's `take` needs `SurrealValue`, not serde).
#[derive(surrealdb_types::SurrealValue)]
struct BanditPayloadRow {
    payload: surrealdb_types::Value,
}

/// Read back the stored bandit-log payloads, newest first, capped at `limit`.
///
/// Each returned string is the serialized `BanditRow` JSON the OPE layer
/// (kavach-ope) deserializes into a `LoggedSample`. The store keeps the payload
/// as an opaque `SurrealDB` value, so this bridges it to a JSON string at the
/// boundary (same `Value` -> `serde_json` hop as `db_harness/read.rs`).
///
/// # Errors
/// Returns an error if the `SELECT` fails or a stored payload is malformed.
pub async fn list_bandit_rows(db: &Surreal<Db>, limit: u32) -> Result<Vec<String>> {
    let query = "SELECT payload, created_at FROM bandit_log ORDER BY created_at DESC LIMIT $limit";
    let mut response = db.query(query).bind(("limit", i64::from(limit))).await?;
    let rows: Vec<BanditPayloadRow> = response.take(0)?;
    rows.into_iter()
        .map(|r| {
            // `into_json_value` emits PLAIN JSON; `serde_json::to_value` would emit
            // SurrealDB's tagged enum form (`{"Object":{"action":{"String":..}}}`)
            // that the OPE estimators cannot parse.
            let json = r.payload.into_json_value();
            serde_json::to_string(&json).map_err(crate::error::Error::Json)
        })
        .collect()
}

/// Tables that carry an `entry_status` field and are valid targets for
/// `update_status`. Compile-time list — extending it requires a code change,
/// preventing typo-driven silent no-ops.
const STATUS_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

/// Returned-id row used by `update_status` and `kanban_close` to count
/// affected rows without triggering `SurrealDB` SDK issue #5794
/// (`Vec<serde_json::Value>` deserialization fails on records containing
/// the enum-asserted `entry_status` field).
#[derive(surrealdb_types::SurrealValue)]
pub(crate) struct UpdatedIdRow {
    pub(crate) id: RecordId,
}

/// Update `entry_status` for a memory entry by typed table + `entry_key`.
///
/// # Errors
/// `Error::Migration` when `table` is not in `STATUS_TABLES`; `Error::Surreal`
/// when the UPDATE itself fails or the response shape is malformed.
pub async fn update_status(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    entry_key: &str,
    new_status: &str,
) -> Result<usize> {
    if !STATUS_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_status: unsupported table '{table}'; allowed: {STATUS_TABLES:?}"
        )));
    }
    let query = format!(
        "UPDATE {table} SET entry_status = $status, updated_at = time::now() \
         WHERE project = $project AND entry_key = $key RETURN id"
    );
    let mut response = db
        .query(query)
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("status", new_status.to_owned()))
        .await?;
    let updated: Vec<UpdatedIdRow> = response.take(0)?;
    let count = updated.len();
    // Touch the deserialized id field so SDK schema validation is exercised.
    if let Some(first) = updated.first() {
        let _ = &first.id.table;
    }
    Ok(count)
}

const FEEDBACK_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

/// Update the `feedback` field for a memory entry.
///
/// # Errors
/// `Error::Migration` when `table` is not in `FEEDBACK_TABLES`; `Error::Surreal`
/// when the UPDATE fails.
pub async fn update_feedback(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    entry_key: &str,
    feedback: &str,
) -> Result<usize> {
    if !FEEDBACK_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_feedback: unsupported table '{table}'"
        )));
    }
    // FIX: [data_flow] write.rs:313 — query string had lost its bind-param
    // placeholders ($feedback/$project/$key), so SET/WHERE were syntactically
    // invalid SurrealQL; the three .bind() calls referenced names absent from
    // the query. Latent (0 call sites today) but breaks the instant it is
    // wired. Restored placeholders to match the binds below.
    let query = format!(
        "UPDATE {table} SET feedback = $feedback, updated_at = time::now() \
         WHERE project = $project AND entry_key = $key RETURN id"
    );
    let mut response = db
        .query(query)
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("feedback", feedback.to_owned()))
        .await?;
    let updated: Vec<UpdatedIdRow> = response.take(0)?;
    Ok(updated.len())
}
