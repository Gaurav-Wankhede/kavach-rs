// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)
use super::full::upsert_entry_full;
use crate::error::Result;
use kavach_types::Priority;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;

/// Upsert entry + event log in one txn (no entity graph).
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
