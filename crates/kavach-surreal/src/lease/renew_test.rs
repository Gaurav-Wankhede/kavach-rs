//! Proof that the renewal janitor extends only live, in-progress leases and
//! leaves finished / released / lapsed ones alone.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::renew_active_leases;
use crate::open_memory;
use chrono::{Duration, Utc};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

/// Seed one roadmap card with explicit lease columns + status.
async fn seed_card(
    db: &Surreal<Db>,
    id: &str,
    occupied_by: Option<&str>,
    until_offset_secs: i64,
    status: &str,
) {
    let until =
        occupied_by.and_then(|_| Utc::now().checked_add_signed(Duration::seconds(until_offset_secs)));
    db.query(
        "CREATE type::record('roadmap', $id) SET \
         occupied_by=$ob, occupied_until=$u, occupied_epoch=1, \
         entry_status=$st, entry_key=$id, project='p', title='t', content='c', \
         updated_at=time::now()",
    )
    .bind(("id", id.to_owned()))
    .bind(("ob", occupied_by.map(ToOwned::to_owned)))
    .bind(("u", until))
    .bind(("st", status.to_owned()))
    .await
    .expect("seed card");
}

async fn until_of(db: &Surreal<Db>, id: &str) -> Option<chrono::DateTime<Utc>> {
    #[derive(surrealdb_types::SurrealValue)]
    struct Row {
        occupied_until: Option<chrono::DateTime<Utc>>,
    }
    let row: Option<Row> = db
        .query("SELECT occupied_until FROM type::record('roadmap', $id)")
        .bind(("id", id.to_owned()))
        .await
        .expect("select")
        .take(0)
        .expect("take");
    row.and_then(|r| r.occupied_until)
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
