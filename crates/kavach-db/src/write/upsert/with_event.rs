// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)
use super::full::upsert_entry_full;
use crate::error::Result;
use crate::db::DbStore;
use kavach_types::Priority;

/// Upsert entry + event log in one txn (no entity graph).
#[bon::builder]
pub async fn upsert_entry_with_event(
    db: &DbStore,
    category: &str,
    project_id: &str,
    entry_key: &str,
    title: &str,
    content: &str,
    event_source: &str,
    priority: Option<Priority>,
) -> Result<String> {
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
