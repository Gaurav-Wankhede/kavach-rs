// U1 umbrella proofs: mistake + loophole share the anti_pattern/event graph tier,
// distinguished by properties.family. SOURCE: decision.loophole-mistake-umbrella.
use super::{
    FAMILY_LOOPHOLE, FAMILY_MISTAKE, append_loophole_event, append_mistake_event,
    upsert_anti_pattern, upsert_anti_pattern_with_family,
};
use crate::error::Result;
use crate::open_memory;
use crate::db::DbStore;

async fn family_of(db: &DbStore, entity_type: &str) -> Result<Option<String>> {
    let mut resp = db
        .exec_params(
            "SELECT json_extract(properties, '$.family') AS family \
             FROM entities WHERE entity_type = ?1 LIMIT 1",
            libsql::params![entity_type.to_owned()],
        )
        .await
        .expect("query");
    let row = resp.next().await.expect("next");
    Ok(row.and_then(|r| r.get::<Option<String>>(0).expect("family")))
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
        family_of(&db, "mistake_event")
            .await
            .expect("family")
            .as_deref(),
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
        0,
    )
    .await
    .expect("append loophole");
    assert_eq!(
        family_of(&db, "loophole_event")
            .await
            .expect("family")
            .as_deref(),
        Some(FAMILY_LOOPHOLE)
    );
}

#[tokio::test]
async fn empty_loophole_dimension_is_rejected() {
    let db = open_memory().await.expect("open in-memory db");
    assert!(
        append_loophole_event(&db, "", "site", "sess", None, 0)
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
    let read = |name: &'static str| {
        let db = db.clone();
        async move {
            let mut r = db
                .exec_params(
                    "SELECT json_extract(properties, '$.family') AS family FROM entities \
                     WHERE entity_type = 'anti_pattern' AND name = ?1 LIMIT 1",
                    libsql::params![name.to_owned()],
                )
                .await
                .expect("q");
            let row = r.next().await.expect("next");
            row.and_then(|x| x.get::<Option<String>>(0).expect("family"))
        }
    };
    assert_eq!(read("ap-default").await.as_deref(), Some(FAMILY_MISTAKE));
    assert_eq!(read("ap-loop").await.as_deref(), Some(FAMILY_LOOPHOLE));
}
