//! Proof that the crash-orphan reclaim resets only abandoned `in_progress`
//! cards (lapsed lease OR no lease at all) back to `todo`, and leaves a live
//! holder, a finished card, and an already-`todo` card untouched.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::reclaim_orphaned_in_progress;
use crate::open_memory;
use crate::turso::TursoStore;
use chrono::{Duration, Utc};

/// Seed one roadmap card. `until_offset_secs` is relative to now; pass `None`
/// for `occupied_by` to leave the lease columns unset (the CAS-claim shape).
async fn seed_card(
    db: &TursoStore,
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
            crate::turso::now_rfc3339(),
            status.to_owned()
        ],
    )
    .await
    .expect("seed card");
}

async fn status_of(db: &TursoStore, id: &str) -> String {
    let mut rows = db
        .exec_params(
            "SELECT entry_status FROM memories \
             WHERE project = 'p' AND category = 'roadmap' AND entry_key = ?1",
            libsql::params![id.to_owned()],
        )
        .await
        .expect("select");
    let row = rows.next().await.expect("next").expect("row exists");
    row.get::<String>(0).expect("get")
}

#[tokio::test]
async fn reclaims_only_orphaned_in_progress() {
    let db = open_memory().await.expect("memory db");
    // live holder (lease far in the future) -> NOT reclaimed
    seed_card(&db, "live", Some("sess_a"), 300, "in_progress").await;
    // crashed holder, lease lapsed -> RECLAIMED to todo
    seed_card(&db, "crashed", Some("sess_b"), -30, "in_progress").await;
    // CAS-claimed, no lease at all -> RECLAIMED (occupied_until = NONE arm)
    seed_card(&db, "noleased", None, 0, "in_progress").await;
    // finished card -> NOT reclaimed
    seed_card(&db, "done", Some("sess_c"), -30, "done").await;
    // already todo -> NOT touched
    seed_card(&db, "open", None, 0, "todo").await;

    let reclaimed = reclaim_orphaned_in_progress(&db).await.expect("reclaim");

    assert_eq!(reclaimed, 2, "exactly the two orphans are reclaimed");
    assert_eq!(
        status_of(&db, "live").await,
        "in_progress",
        "live holder kept"
    );
    assert_eq!(
        status_of(&db, "crashed").await,
        "todo",
        "lapsed lease reclaimed"
    );
    assert_eq!(
        status_of(&db, "noleased").await,
        "todo",
        "no-lease orphan reclaimed"
    );
    assert_eq!(
        status_of(&db, "done").await,
        "done",
        "finished card untouched"
    );
    assert_eq!(
        status_of(&db, "open").await,
        "todo",
        "already-todo untouched"
    );
}

#[tokio::test]
async fn reclaim_is_idempotent() {
    let db = open_memory().await.expect("memory db");
    seed_card(&db, "crashed", Some("sess_b"), -30, "in_progress").await;
    let first = reclaim_orphaned_in_progress(&db).await.expect("first");
    let second = reclaim_orphaned_in_progress(&db).await.expect("second");
    assert_eq!(first, 1, "first sweep reclaims the orphan");
    assert_eq!(
        second, 0,
        "second sweep finds it already todo — no double reset"
    );
}
