// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)
use crate::error::Result;
use kavach_types::Priority;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;

use super::graph_stmts::append_entity_graph_stmts;

/// Atomic upsert: entry + event + graph in one txn.
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
    exec_prompt: Option<&str>,
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
    let pk = crate::key_str::project_key_str(project_id);
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
    let exec_prompt_clause: &str = if table == "roadmap" {
        ", exec_prompt = IF $exec_prompt != NONE THEN $exec_prompt ELSE exec_prompt END"
    } else {
        ""
    };
    writeln!(
        q,
        "UPSERT $eid \
            SET project = $project, category = '{table}', \
            entry_key = $key, title = $title, content = $content, \
            status = 'active'{priority_clause}{exec_prompt_clause}, updated_at = time::now() RETURN id;"
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

    let project_name = crate::key_str::project_key_str(project_id);
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
            .bind(("exec_prompt", exec_prompt.map(str::to_owned)))
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
