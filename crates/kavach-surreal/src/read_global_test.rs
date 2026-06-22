//! Red-Green proof: `list_by_project_or_global` surfaces global (null-project)
//! rows alongside project-scoped ones — the fix for invisible mistake ledger
//! entries. SOURCE: roadmap.mistake-ledger-resurrect (DB-proven: mistake.* pattern
//! rows persist with project = NONE, so strict `project = $p` returns empty).

use crate::{apply_schema, list_by_project, list_by_project_or_global, open_memory};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;

/// Insert a `pattern` row with an explicit project, or NONE when `proj` is empty.
async fn insert_pattern(db: &Surreal<Db>, proj: Option<&RecordId>, key: &str) {
    let q = if proj.is_some() {
        "CREATE pattern SET project = $project, entry_key = $key, title = 't', content = 'c'"
    } else {
        "CREATE pattern SET project = NONE, entry_key = $key, title = 't', content = 'c'"
    };
    let mut query = db.query(q).bind(("key", key.to_owned()));
    if let Some(p) = proj {
        query = query.bind(("project", p.clone()));
    }
    query.await.expect("insert pattern");
}

#[tokio::test]
async fn project_scoped_read_misses_global_rows() {
    // Establishes the BUG: strict list_by_project cannot see a null-project row.
    let db = open_memory().await.expect("open mem");
    apply_schema(&db).await.expect("schema");
    let proj = RecordId::new("project", "kavach-rs");
    insert_pattern(&db, None, "mistake.global.row").await;
    let rows = list_by_project(&db, "pattern", &proj).await.expect("list");
    assert!(
        !rows.iter().any(|r| r.entry_key == "mistake.global.row"),
        "strict project read must NOT surface a global row (proves the bug)"
    );
}

#[tokio::test]
async fn or_global_read_surfaces_both_scoped_and_global() {
    let db = open_memory().await.expect("open mem");
    apply_schema(&db).await.expect("schema");
    let proj = RecordId::new("project", "kavach-rs");
    insert_pattern(&db, Some(&proj), "mistake.scoped.row").await;
    insert_pattern(&db, None, "mistake.global.row").await;
    let rows = list_by_project_or_global(&db, "pattern", &proj)
        .await
        .expect("list or-global");
    assert!(
        rows.iter().any(|r| r.entry_key == "mistake.scoped.row"),
        "must surface the project-scoped row"
    );
    assert!(
        rows.iter().any(|r| r.entry_key == "mistake.global.row"),
        "must ALSO surface the global (null-project) row — the fix"
    );
}

#[tokio::test]
async fn or_global_read_excludes_other_projects() {
    // The OR must admit global rows, NOT another project's scoped rows.
    let db = open_memory().await.expect("open mem");
    apply_schema(&db).await.expect("schema");
    let mine = RecordId::new("project", "kavach-rs");
    let other = RecordId::new("project", "someone-else");
    insert_pattern(&db, Some(&other), "mistake.other.row").await;
    let rows = list_by_project_or_global(&db, "pattern", &mine)
        .await
        .expect("list or-global");
    assert!(
        !rows.iter().any(|r| r.entry_key == "mistake.other.row"),
        "another project's scoped row must NOT leak in"
    );
}
