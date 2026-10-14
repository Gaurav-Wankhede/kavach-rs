use crate::error::Result;
use crate::db::DbStore;
use crate::{apply_schema, open_memory, update_status, upsert_entry_full};

struct StatusRow {
    entry_status: Option<String>,
    content: String,
    access_count: i64,
}

async fn read_card(db: &DbStore, key: &str) -> Result<StatusRow> {
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

async fn upsert(db: &DbStore, proj: &str, key: &str, content: &str) {
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
        Some("todo"),
        "create seeds the roadmap lane (K-1: no more NULL lanes)"
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

#[tokio::test]
async fn fresh_roadmap_upsert_seeds_todo_lane() {
    // K-1 root cause: the daemon write path stored entry_status = NULL, so
    // every board had to guess. Creation now seeds the lane.
    let db = open_memory().await.expect("open in-memory db");
    apply_schema(&db).await.expect("schema");
    upsert(&db, "p", "fresh-card", "v1").await;
    let row = read_card(&db, "fresh-card").await.expect("read");
    assert_eq!(row.entry_status.as_deref(), Some("todo"));
}

#[tokio::test]
async fn non_roadmap_upsert_leaves_lane_null() {
    // Decisions/research live their lifecycle in `status`, not the kanban
    // lane — seeding `todo` there would corrupt COALESCE status filters.
    let db = open_memory().await.expect("open in-memory db");
    apply_schema(&db).await.expect("schema");
    upsert_entry_full()
        .db(&db)
        .category("decision")
        .project_id("p")
        .entry_key("d1")
        .title("t")
        .content("c")
        .event_source("test")
        .qualified_name("")
        .references(&[])
        .build_for_call()
        .await
        .expect("upsert");
    let mut resp = db
        .exec_params(
            "SELECT entry_status FROM memories WHERE category = 'decision' AND entry_key = 'd1'",
            (),
        )
        .await
        .expect("select");
    let row = resp.next().await.expect("next").expect("row exists");
    let lane: Option<String> = row.get(0).expect("lane decodes");
    assert_eq!(lane, None);
}

#[tokio::test]
async fn rewrite_heals_legacy_null_lane() {
    // Pre-fix rows carry (status='active', entry_status=NULL): the first
    // re-write fills the lane instead of leaving it null forever.
    let db = open_memory().await.expect("open in-memory db");
    apply_schema(&db).await.expect("schema");
    db.exec_params(
        "INSERT INTO memories (project, category, entry_key, title, content, status, created_at, updated_at) \
         VALUES ('p', 'roadmap', 'legacy', 't', 'c', 'active', datetime('now'), datetime('now'))",
        (),
    )
    .await
    .expect("legacy seed");
    upsert(&db, "p", "legacy", "v2").await;
    let row = read_card(&db, "legacy").await.expect("read");
    assert_eq!(row.entry_status.as_deref(), Some("todo"));
}
