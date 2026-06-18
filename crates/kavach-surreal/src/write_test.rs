// Proves the preserve-on-update contract that kills kanban status-drift:
// CREATE gets the schema DEFAULT ('todo'); a later content re-write of the
// same key (the `db write --update-key` path) must NOT reset `entry_status`
// — completed work flipping back to 'todo' made the board lie and the loop
// dispatch phantom tasks.
use crate::{apply_schema, open_memory, update_status, upsert_entry_full};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(SurrealValue)]
struct StatusRow {
    entry_status: String,
    content: String,
    access_count: i64,
}

async fn read_card(db: &Surreal<Db>, key: &str) -> StatusRow {
    let mut resp = db
        .query("SELECT entry_status, content, access_count FROM roadmap WHERE entry_key = $key")
        .bind(("key", key.to_owned()))
        .await
        .expect("select");
    let rows: Vec<StatusRow> = resp.take(0).expect("rows");
    rows.into_iter().next().expect("card exists")
}

async fn upsert(db: &Surreal<Db>, proj: &RecordId, key: &str, content: &str) {
    upsert_entry_full()
        .db(db)
        .category("roadmap")
        .project_id(proj)
        .entry_key(key)
        .title("t")
        .content(content)
        .event_source("test")
        .qualified_name("")
        .references(&[])
        .build_for_call()
        .await
        .expect("upsert");
}

#[tokio::test]
async fn update_preserves_entry_status_and_access_count() {
    let db = open_memory().await.expect("open in-memory db");
    apply_schema(&db).await.expect("schema");
    let proj = RecordId::new("project", "test-proj");

    // CREATE: schema DEFAULT applies — fresh card lands at 'todo'.
    upsert(&db, &proj, "card-1", "v1").await;
    let created = read_card(&db, "card-1").await;
    assert_eq!(created.entry_status, "todo", "create defaults to todo");
    assert_eq!(created.access_count, 0, "create defaults access_count 0");

    // Lifecycle transition through the dedicated setter.
    let n = update_status(&db, "roadmap", &proj, "card-1", "verified")
        .await
        .expect("status update");
    assert_eq!(n, 1, "one row transitioned");

    // Bump access_count so preservation (not re-default) is observable.
    db.query("UPDATE roadmap SET access_count = 7 WHERE entry_key = 'card-1'")
        .await
        .expect("bump access_count");

    // UPDATE (the `db write --update-key` path): content changes, status and
    // access_count MUST survive — this was the status-drift bug.
    upsert(&db, &proj, "card-1", "v2 — parked note appended").await;
    let updated = read_card(&db, "card-1").await;
    assert_eq!(
        updated.content, "v2 — parked note appended",
        "content updated"
    );
    assert_eq!(
        updated.entry_status, "verified",
        "re-write must NOT reset a verified card to todo"
    );
    assert_eq!(
        updated.access_count, 7,
        "re-write must NOT zero access_count"
    );
}
