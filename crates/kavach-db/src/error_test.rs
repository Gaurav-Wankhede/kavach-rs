// Proves the table-not-found catch is anchored: a missing TABLE matches (so the
// fresh-graph empty case stays fail-open), but a different `does not exist` error
// (an undefined function) does NOT — so a real malformed query propagates as Err
// instead of masquerading as an empty result.
use super::is_missing_table_error;
use crate::open_memory;

#[tokio::test]
async fn missing_table_matches_but_other_does_not_exist_errors_do_not() {
    let db = open_memory().await.expect("open in-memory db");

    // SELECT from a never-created table raises "The table '...' does not exist".
    let table_err = db
        .query("SELECT * FROM never_created_table")
        .await
        .and_then(|mut r| r.take::<Vec<i64>>(0))
        .expect_err("missing table must error");
    assert!(
        is_missing_table_error(&table_err),
        "table-not-found must match, got: {table_err}"
    );

    // An undefined function raises "The function '...' does not exist" — a sibling
    // does-not-exist error that must NOT be swallowed as an empty result.
    let fn_err = db
        .query("RETURN fn::definitely_not_defined()")
        .await
        .and_then(|mut r| r.take::<Vec<i64>>(0))
        .expect_err("undefined function must error");
    assert!(
        !is_missing_table_error(&fn_err),
        "a non-table does-not-exist error must propagate, got: {fn_err}"
    );
}
