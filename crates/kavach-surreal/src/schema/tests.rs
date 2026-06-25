//! Schema apply + citation evidence-gate ASSERT tests.
use crate::{apply_schema, open_memory};

async fn db_with_project() -> surrealdb::Surreal<surrealdb::engine::any::Any> {
    let db = open_memory().await.expect("mem db");
    apply_schema(&db).await.expect("schema applies with citation table");
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
async fn citation_with_empty_url_is_rejected() {
    let db = db_with_project().await;
    let rejected = db
        .query(
            "CREATE citation SET project = project:p, entry_key = 'c2', name = 'X', \
             metadata = [{ slug: 's', url: '' }]",
        )
        .await
        .map_or(true, |resp| resp.check().is_err());
    assert!(rejected, "empty metadata url must fail the evidence-gate ASSERT");
}
