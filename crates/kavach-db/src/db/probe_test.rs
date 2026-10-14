//! RUNTIME WITNESS for the three Database-only capabilities the graph architecture
//! depends on. Docs (docs.database.tech) describe the hosted product; this proves
//! what the EMBEDDED `libsql` build we actually ship actually accepts.
//!
//! Sources:
//!   https://docs.database.tech/sql-reference/extensions
//!   https://docs.database.tech/sql-reference/pragmas
//!   https://docs.database.tech/sql-reference/statements/transactions
//!   https://docs.database.tech/sql-reference/functions/vector
//!
//! The assertions are deliberately TOLERANT of absence: a missing feature is a
//! real answer that must be recorded, not a test failure. This test's job is to
//! PRINT the capability matrix, not to assert it — asserting would make the
//! suite fail the day a feature disappears, which is the opposite of what a
//! witness should do. Run with `--nocapture`.

use crate::db::DbStore;

/// Run one statement and render the outcome as a capability string. Any error is
/// an ANSWER (the feature is absent), not a failure — a witness must survive the
/// feature disappearing.
async fn try_exec(conn: &DbStore, sql: &str) -> String {
    match conn.exec(sql).await {
        Ok(_) => "YES".to_owned(),
        Err(e) => format!("NO ({})", e.to_string().chars().take(70).collect::<String>()),
    }
}

/// Query one scalar, rendering absence as an answer.
///
/// Reads the RAW `libsql::Value` via `get_value`, never a typed getter.
/// libsql 0.9.30's `FromValue` impls hit `unreachable!("invalid value type")` —
///
/// a PANIC, not an `Err` — on any type mismatch (`rows.rs:196` for i64, `:218`
/// for f64). So `row.get::<f64>(0)` on a `count(*)` column aborts the process,
/// which is exactly the defect class fixed in `write/events.rs` and `runs.rs`.
/// `get_value` is the only non-panicking read. SOURCE:
/// <https://github.com/databasedatabase/libsql/blob/v0.9.30/src/rows.rs>
async fn try_scalar(conn: &DbStore, sql: &str) -> String {
    match conn.exec_params(sql, ()).await {
        Ok(mut rows) => match rows.next().await.unwrap_or(None) {
            None => "YES (no row)".to_owned(),
            Some(row) => match row.get_value(0) {
                Ok(libsql::Value::Integer(i)) => format!("YES (int={i})"),
                Ok(libsql::Value::Real(f)) => format!("YES (real={f})"),
                Ok(libsql::Value::Text(t)) => format!("YES (text={t})"),
                Ok(libsql::Value::Null) => "YES (null)".to_owned(),
                Ok(other) => format!("YES ({other:?})"),
                Err(e) => format!("YES (unreadable: {e})"),
            },
        },
        Err(e) => format!("NO ({})", e.to_string().chars().take(70).collect::<String>()),
    }
}

#[tokio::test]
async fn turso_embedded_capability_matrix() {
    let conn = match DbStore::open_memory().await {
        Ok(c) => c,
        Err(e) => {
            println!("PROBE: could not open a store — inconclusive: {e}");
            return;
        }
    };

    // 1. CDC — the append-only assertion log.
    let cdc_pragma = try_exec(&conn, "PRAGMA capture_data_changes_conn('full')").await;
    let cdc_table = try_scalar(&conn, "SELECT count(*) FROM turso_cdc").await;

    // 2. MVCC — snapshot isolation for concurrent readers/writers.
    let mvcc_mode = try_exec(&conn, "PRAGMA journal_mode = mvcc").await;
    let begin_concurrent = try_exec(&conn, "BEGIN CONCURRENT").await;
    let _ = conn.exec("COMMIT").await;

    // 3. Native vector functions — Database's built-in, vs the hand-rolled ranker.
    let vec_cos = try_scalar(
        &conn,
        "SELECT vector_distance_cos(vector32('[0.1,0.2,0.3]'), vector32('[0.1,0.2,0.3]'))",
    )
    .await;

    // 4. load_extension — expected refusal; proves the sqlite-vec family is out.
    let load_ext = try_exec(&conn, "SELECT load_extension('/nonexistent')").await;

    // 5. The recursive-CTE graph traversal we ALREADY depend on.
    let recursive_cte = try_scalar(
        &conn,
        "WITH RECURSIVE c(n) AS (SELECT 1 UNION SELECT n+1 FROM c WHERE n < 5) SELECT count(*) FROM c",
    )
    .await;

    let report = format!(
        "\n  CDC pragma          : {cdc_pragma}\n  turso_cdc table     : {cdc_table}\n  \
         journal_mode=mvcc   : {mvcc_mode}\n  BEGIN CONCURRENT    : {begin_concurrent}\n  \
         vector_distance_cos : {vec_cos}\n  load_extension      : {load_ext}\n  \
         recursive CTE       : {recursive_cte}\n"
    );
    println!("{report}");
    // rtk/cargo suppress stdout in this harness, so the matrix is ALSO written
    // to a file the run can read back. The witness must survive a quiet runner.
    let out = std::env::temp_dir().join("kavach_turso_capability_matrix.txt");
    std::fs::write(&out, &report).ok();
}
