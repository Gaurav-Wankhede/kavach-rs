//! Schema apply + citation evidence-gate tests.
use crate::turso::TursoStore;
use crate::{apply_schema, open_memory};

async fn db_with_project() -> TursoStore {
    let db = open_memory().await.expect("mem db");
    apply_schema(&db)
        .await
        .expect("schema applies with citation table");
    db.exec_params("INSERT INTO projects (slug, display) VALUES ('p', 'p')", ())
        .await
        .expect("seed project");
    db
}

#[tokio::test]
async fn citation_with_url_is_accepted() {
    let db = db_with_project().await;
    db.exec_params(
        "INSERT INTO citations (project, entry_key, name, slug, url, created_at, updated_at) \
         VALUES ('p', 'c1', 'TursoStoreDB', 'records', 'https://surrealdb.com/docs', ?1, ?1)",
        libsql::params![crate::turso::now_rfc3339()],
    )
    .await
    .expect("a citation with a non-empty url is accepted");
}

#[tokio::test]
async fn roadmap_exec_prompt_persists_and_reads_back() {
    let db = db_with_project().await;
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, exec_prompt, created_at, updated_at) \
         VALUES ('p', 'roadmap', 'r1', 't', 'c', \
                 'You are implementing X. Files: a.rs. Done when tests pass.', ?1, ?1)",
        libsql::params![crate::turso::now_rfc3339()],
    )
    .await
    .expect("a roadmap row with exec_prompt is accepted");
    let mut resp = db
        .exec_params(
            "SELECT exec_prompt FROM memories WHERE project = 'p' AND category = 'roadmap' AND entry_key = 'r1'",
            (),
        )
        .await
        .expect("read ran");
    let row = resp.next().await.expect("next").expect("row exists");
    let got: String = row.get(0).expect("get");
    assert_eq!(
        got, "You are implementing X. Files: a.rs. Done when tests pass.",
        "exec_prompt round-trips verbatim"
    );
}

#[tokio::test]
async fn roadmap_without_exec_prompt_is_accepted_as_none() {
    let db = db_with_project().await;
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, created_at, updated_at) \
         VALUES ('p', 'roadmap', 'r2', 't', 'c', ?1, ?1)",
        libsql::params![crate::turso::now_rfc3339()],
    )
    .await
    .expect("exec_prompt is optional — a row without it is valid");
}

#[tokio::test]
async fn citation_with_empty_url_is_rejected() {
    let db = db_with_project().await;
    let rejected = crate::citation_upsert(
        &db,
        &crate::UpsertCitation {
            project: "p".to_owned(),
            entry_key: "c2",
            name: "X",
            metadata: vec![crate::CitationMeta::new("s".to_owned(), String::new())],
        },
    )
    .await
    .is_err();
    assert!(
        rejected,
        "empty metadata url must fail the evidence-gate validation"
    );
}
