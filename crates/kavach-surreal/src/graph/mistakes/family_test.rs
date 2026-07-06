// U1 umbrella proofs: mistake + loophole share the anti_pattern/event graph tier,
// distinguished by properties.family. SOURCE: decision.loophole-mistake-umbrella.
use super::{
    FAMILY_LOOPHOLE, FAMILY_MISTAKE, append_loophole_event, append_mistake_event,
    upsert_anti_pattern, upsert_anti_pattern_with_family,
};
use crate::open_memory;
use surrealdb_types::SurrealValue;

#[derive(SurrealValue)]
struct FamilyRow {
    family: String,
}

async fn family_of(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    entity_type: &str,
) -> Option<String> {
    let q = "SELECT properties.family AS family FROM entity WHERE entity_type = $t LIMIT 1";
    let mut resp = db
        .query(q)
        .bind(("t", entity_type.to_owned()))
        .await
        .expect("query");
    let row: Option<FamilyRow> = resp.take(0).expect("take");
    row.map(|r| r.family)
}

#[tokio::test]
async fn mistake_event_is_tagged_family_mistake() {
    let db = open_memory().await.expect("open in-memory db");
    append_mistake_event(
        &db,
        "shallow_verdict",
        "cite file:line",
        "banned",
        "sess",
        Some("proj"),
        0,
    )
    .await
    .expect("append mistake");
    assert_eq!(
        family_of(&db, "mistake_event").await.as_deref(),
        Some(FAMILY_MISTAKE)
    );
}

#[tokio::test]
async fn loophole_event_is_tagged_family_loophole() {
    let db = open_memory().await.expect("open in-memory db");
    append_loophole_event(
        &db,
        "injection",
        "src/x.py:12 — os.system",
        "sess",
        Some("proj"),
    )
    .await
    .expect("append loophole");
    assert_eq!(
        family_of(&db, "loophole_event").await.as_deref(),
        Some(FAMILY_LOOPHOLE)
    );
}

#[tokio::test]
async fn empty_loophole_dimension_is_rejected() {
    let db = open_memory().await.expect("open in-memory db");
    assert!(
        append_loophole_event(&db, "", "site", "sess", None)
            .await
            .is_err(),
        "an empty dimension must be refused, never a blank-gate row"
    );
}

#[tokio::test]
async fn anti_pattern_default_family_is_mistake_overlay_is_loophole() {
    let db = open_memory().await.expect("open in-memory db");
    // Back-compat path defaults to mistake.
    upsert_anti_pattern(&db, "ap-default", "g1", "fix")
        .await
        .expect("default");
    // Explicit loophole family.
    upsert_anti_pattern_with_family(&db, "ap-loop", "injection", "parameterize", FAMILY_LOOPHOLE)
        .await
        .expect("loophole family");
    let q = "SELECT properties.family AS family FROM entity \
             WHERE entity_type = 'anti_pattern' AND name = $n LIMIT 1";
    let read = |name: &'static str| {
        let db = db.clone();
        async move {
            let mut r = db.query(q).bind(("n", name)).await.expect("q");
            let row: Option<FamilyRow> = r.take(0).expect("take");
            row.map(|x| x.family)
        }
    };
    assert_eq!(read("ap-default").await.as_deref(), Some(FAMILY_MISTAKE));
    assert_eq!(read("ap-loop").await.as_deref(), Some(FAMILY_LOOPHOLE));
}
