use crate::error::Result;
use crate::turso::TursoStore;
use crate::{apply_schema, open_memory, update_status, upsert_entry_full};

struct StatusRow {
    entry_status: Option<String>,
    content: String,
    access_count: i64,
}

async fn read_card(db: &TursoStore, key: &str) -> Result<StatusRow> {
    let mut resp = db
        .exec_params(
            "SELECT entry_status, content, access_count FROM memories \
             WHERE category = 'roadmap' AND entry_key = ?1",
            libsql::params![key.to_owned()],
        )
        .await
        .expect("select");
    let row = resp.next().await.expect("next").expect("card exists");
    Ok(StatusRow {
        entry_status: row.get(0).expect("entry_status"),
        content: row.get(1).expect("content"),
        access_count: row.get(2).expect("access_count"),
    })
}

async fn upsert(db: &TursoStore, proj: &str, key: &str, content: &str) {
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
    let proj = "test-proj".to_owned();

    upsert(&db, &proj, "card-1", "v1").await;
    let created = read_card(&db, "card-1").await.expect("read created");
    assert_eq!(
        created.entry_status.as_deref(),
        None,
        "create leaves entry_status unset"
    );
    assert_eq!(created.access_count, 0, "create defaults access_count 0");

    let n = update_status(&db, "roadmap", &proj, "card-1", "verified")
        .await
        .expect("status update");
    assert_eq!(n, 1, "one row transitioned");

    db.exec_params(
        "UPDATE memories SET access_count = 7 \
         WHERE category = 'roadmap' AND entry_key = 'card-1'",
        (),
    )
    .await
    .expect("bump access_count");

    upsert(&db, &proj, "card-1", "v2 — parked note appended").await;
    let updated = read_card(&db, "card-1").await.expect("read updated");
    assert_eq!(
        updated.content, "v2 — parked note appended",
        "content updated"
    );
    assert_eq!(
        updated.entry_status.as_deref(),
        Some("verified"),
        "re-write must NOT reset a verified card to todo"
    );
    assert_eq!(
        updated.access_count, 7,
        "re-write must NOT zero access_count"
    );
}
