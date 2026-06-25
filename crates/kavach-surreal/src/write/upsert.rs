use crate::error::Result;
use kavach_types::Priority;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;
use super::status::UpdatedIdRow;

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
    let pk = format!("{:?}", project_id.key);
    let rid = RecordId::new(category, format!("{pk}:{entry_key}"));
    let query = match category {
        "decision" => {
            "LET $eid = (SELECT VALUE id FROM decision WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'decision', entry_key = $key, title = $title, content = $content, status = 'active', priority = IF $priority != NONE THEN $priority ELSE priority END, updated_at = time::now() RETURN id"
        }
        "research" => {
            "LET $eid = (SELECT VALUE id FROM research WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'research', entry_key = $key, title = $title, content = $content, status = 'active', updated_at = time::now() RETURN id"
        }
        "roadmap" => {
            "LET $eid = (SELECT VALUE id FROM roadmap WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'roadmap', entry_key = $key, title = $title, content = $content, status = 'active', priority = IF $priority != NONE THEN $priority ELSE priority END, updated_at = time::now() RETURN id"
        }
        "pattern" => {
            "LET $eid = (SELECT VALUE id FROM pattern WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'pattern', entry_key = $key, title = $title, content = $content, status = 'active', updated_at = time::now() RETURN id"
        }
        "app_spec" => {
            "LET $eid = (SELECT VALUE id FROM app_spec WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'app_spec', entry_key = $key, title = $title, content = $content, status = 'active', updated_at = time::now() RETURN id"
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
            status = 'active'{priority_clause}, updated_at = time::now() RETURN id;"
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
    crate::retry::with_retry(|| async {
        let response = db
            .query(q.clone())
            .bind(("rid", rid.clone()))
            .bind(("project", project_id.clone()))
            .bind(("key", entry_key.to_owned()))
            .bind(("title", title.to_owned()))
            .bind(("content", content.to_owned()))
            .bind(("priority", priority_i64))
            .bind(("source", event_source.to_owned()))
            .bind(("qname", qualified_name.to_owned()))
            .bind(("project_name", project_name.clone()))
            .bind(("refs", references.to_vec()))
            .await?;
        response.check().map_err(crate::error::Error::Surreal)?;
        Ok(rid_returned.clone())
    })
    .await
}
