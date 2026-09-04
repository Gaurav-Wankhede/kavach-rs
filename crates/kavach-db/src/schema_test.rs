//! Schema apply + citation evidence-gate ASSERT tests.
use crate::{apply_schema, open_memory};

async fn db_with_project() -> surrealdb::Surreal<surrealdb::engine::any::Any> {
    let db = open_memory().await.expect("mem db");
    apply_schema(&db)
        .await
        .expect("schema applies with citation table");
    db.query("CREATE project:p SET slug = 'p', name = 'p'")
        .await
        .expect("seed project");
    db
}

#[tokio::test]
async fn citation_with_url_is_accepted() {
    let db = db_with_project().await;
    db.query(
        "CREATE citation SET project = project:p, entry_key = 'c1', name = 'SurrealDB', \
         metadata = [{ slug: 'records', url: 'https://surrealdb.com/docs' }]",
    )
    .await
    .expect("query ran")
    .check()
    .expect("a citation with a non-empty url is accepted");
}

#[tokio::test]
async fn roadmap_exec_prompt_persists_and_reads_back() {
    let db = db_with_project().await;
    db.query(
        "CREATE roadmap SET project = project:p, entry_key = 'r1', title = 't', \
         content = 'c', exec_prompt = 'You are implementing X. Files: a.rs. Done when tests pass.'",
    )
    .await
    .expect("query ran")
    .check()
    .expect("a roadmap row with exec_prompt is accepted");
    let mut resp = db
        .query("SELECT VALUE exec_prompt FROM roadmap WHERE entry_key = 'r1'")
        .await
        .expect("read ran");
    let got: Option<String> = resp.take(0).expect("take");
    assert_eq!(
        got.as_deref(),
        Some("You are implementing X. Files: a.rs. Done when tests pass."),
        "exec_prompt round-trips verbatim"
    );
}

#[tokio::test]
async fn roadmap_without_exec_prompt_is_accepted_as_none() {
    let db = db_with_project().await;
    db.query(
        "CREATE roadmap SET project = project:p, entry_key = 'r2', title = 't', content = 'c'",
    )
    .await
    .expect("query ran")
    .check()
    .expect("exec_prompt is optional — a row without it is valid");
}

#[tokio::test]
async fn citation_with_empty_url_is_rejected() {
    let db = db_with_project().await;
    let rejected = db
        .query(
            "CREATE citation SET project = project:p, entry_key = 'c2', name = 'X', \
             metadata = [{ slug: 's', url: '' }]",
        )
        .await
        .map_or(true, |resp| resp.check().is_err());
    assert!(
        rejected,
        "empty metadata url must fail the evidence-gate ASSERT"
    );
}
