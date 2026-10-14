//! Proof that the renewal janitor extends only live, in-progress leases and
//! leaves finished / released / lapsed ones alone.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::renew_active_leases;
use crate::open_memory;
use crate::db::DbStore;
use chrono::{Duration, Utc};

/// Seed one roadmap card with explicit lease columns + status.
async fn seed_card(
    db: &DbStore,
    id: &str,
    occupied_by: Option<&str>,
    until_offset_secs: i64,
    status: &str,
) {
    let until = occupied_by
        .and_then(|_| Utc::now().checked_add_signed(Duration::seconds(until_offset_secs)));
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, entry_status, \
          occupied_by, occupied_until, occupied_epoch, created_at, updated_at) \
         VALUES ('p', 'roadmap', ?1, 't', 'c', ?5, ?2, ?3, 1, ?4, ?4)",
        libsql::params![
            id.to_owned(),
            occupied_by.map(ToOwned::to_owned),
            until.map(|u| u.to_rfc3339()),
            crate::db::now_rfc3339(),
            status.to_owned()
        ],
    )
    .await
    .expect("seed card");
}

async fn until_of(db: &DbStore, id: &str) -> Option<chrono::DateTime<Utc>> {
    let mut rows = db
        .exec_params(
            "SELECT occupied_until FROM memories \
             WHERE project = 'p' AND category = 'roadmap' AND entry_key = ?1",
            libsql::params![id.to_owned()],
        )
        .await
        .expect("select");
    let row = rows.next().await.expect("next").expect("row exists");
    row.get::<Option<String>>(0).expect("get").and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&Utc))
    })
}

#[tokio::test]
async fn renews_only_live_in_progress_leases() {
    let db = open_memory().await.expect("memory db");
    // live + in_progress  -> renewed (expiry pushed far into the future)
    seed_card(&db, "live", Some("sess_a"), 30, "in_progress").await;
    // live holder but card already done -> NOT renewed
    seed_card(&db, "done", Some("sess_a"), 30, "done").await;
    // released lease (no holder) -> NOT renewed
    seed_card(&db, "free", None, 0, "todo").await;
    // lapsed lease (until in the past) -> NOT resurrected
    seed_card(&db, "lapsed", Some("sess_b"), -30, "in_progress").await;

    let before_live = until_of(&db, "live").await.expect("live until");
    let renewed = renew_active_leases(&db).await.expect("renew");

    assert_eq!(renewed, 1, "exactly the live in_progress lease is renewed");
    let after_live = until_of(&db, "live").await.expect("live until after");
    assert!(
        after_live > before_live,
        "live lease expiry must be pushed forward"
    );
    // The lapsed lease's expiry stays in the past — not renewed back to life.
    let lapsed = until_of(&db, "lapsed").await.expect("lapsed until");
    assert!(
        lapsed < Utc::now(),
        "a lapsed lease is left for the next acquirer"
    );
}
