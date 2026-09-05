// Proves the missing-table case is anchored: a SELECT from a never-created
// table must surface as an Err (SQLite "no such table"), never as a silently
// empty result — the fresh-graph empty case stays fail-open only for tables
// that exist but hold no rows.
use crate::open_memory;

#[tokio::test]
async fn missing_table_errors_instead_of_silently_empty() {
    let db = open_memory().await.expect("open in-memory db");

    // SELECT from a never-created table raises "no such table".
    let table_err = db
        .exec_params("SELECT * FROM never_created_table", ())
        .await
        .expect_err("missing table must error");
    let msg = table_err.to_string();
    assert!(
        msg.contains("no such table"),
        "table-not-found must surface, got: {msg}"
    );
}
