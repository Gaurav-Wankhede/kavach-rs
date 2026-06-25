//! Roundtrip proof for the rag_tree persist+fetch path: tree_upsert -> tree_get.
use super::{TreeGetParams, TreeUpsertParams, tree_get, tree_upsert};
use crate::state::AppState;

async fn state() -> AppState {
    let db = kavach_surreal::open_memory().await.expect("mem db");
    kavach_surreal::apply_schema_engine(&db).await.expect("engine schema");
    AppState::new(db)
}

#[tokio::test]
async fn upsert_then_get_returns_the_persisted_blob() {
    let st = state().await;
    let blob = b"{\"node\":1}\n{\"node\":2}\n".to_vec();
    tree_upsert(
        &st,
        TreeUpsertParams {
            source: "skills".to_owned(),
            built_at: "2026-06-25T00:00:00Z".to_owned(),
            tree_json: blob.clone(),
            source_hash: "deadbeef".to_owned(),
            source_dir: "/skills".to_owned(),
        },
    )
    .await
    .expect("upsert ok");

    let mut raw = st
        .db
        .query("SELECT source, source_hash FROM rag_tree")
        .await
        .expect("raw select");
    let dbg: Vec<serde_json::Value> = raw.take(0).expect("raw take");
    eprintln!("RAW ROWS = {dbg:?}");

    let row = tree_get(&st, TreeGetParams { source: "skills".to_owned() })
        .await
        .expect("get ok")
        .expect("row present after upsert");
    assert_eq!(
        row.tree_json,
        surrealdb_types::Bytes::from(blob.clone()),
        "fetched blob must equal persisted blob"
    );
    assert_eq!(row.source_hash, "deadbeef");
}

#[tokio::test]
async fn get_missing_source_is_none_not_err() {
    let st = state().await;
    let row = tree_get(&st, TreeGetParams { source: "absent".to_owned() })
        .await
        .expect("get ok");
    assert!(row.is_none(), "a missing label fetches None, not an error");
}
