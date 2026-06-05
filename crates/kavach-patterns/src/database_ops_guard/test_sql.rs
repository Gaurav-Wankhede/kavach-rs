//! Tests for SQL-specific database operation detection.

use crate::database_ops_guard::{DbOpsSeverity, detect};

fn k(parts: &[&str]) -> String {
    parts.concat()
}

#[test]
fn select_star_flagged() {
    let q = k(&["SE", "LECT * FROM users"]);
    let src = ["use sqlx; let q = \"", &q, "\";"].concat();
    let r = detect("src/repository/user.rs", &src);
    assert!(r.iter().any(|v| v.pattern == "select-star"));
}

#[test]
fn format_sql_blocked() {
    let kw = k(&["SE", "LECT"]);
    let macro_token = k(&["form", "at", "!"]);
    let src = [
        "use sqlx; let q = ",
        &macro_token,
        "(\"",
        &kw,
        " * FROM t WHERE id = {}\", id);",
    ]
    .concat();
    let r = detect("src/db/query.rs", &src);
    assert!(
        r.iter()
            .any(|v| v.pattern == "format-string-sql" && v.severity == DbOpsSeverity::P0Block)
    );
}

#[test]
fn destructive_drop_blocked() {
    let kw = k(&["DR", "OP TABLE users"]);
    let src = ["-- migration\n", &kw, ";"].concat();
    let r = detect("migrations/001.sql", &src);
    assert!(r.iter().any(|v| v.pattern == "destructive-sql"));
}

#[test]
fn update_without_for_update_advisory() {
    let kw = k(&["UPD", "ATE accounts SET balance = 1"]);
    let src = ["use sqlx; let q = \"", &kw, "\";"].concat();
    let r = detect("src/repository/acct.rs", &src);
    assert!(r.iter().any(|v| v.pattern == "update-without-for-update"));
}

#[test]
fn set_without_local_blocked() {
    let src = r#"use sqlx; let q = "SET app.tenant = $1";"#;
    let r = detect("src/db/ctx.rs", src);
    assert!(r.iter().any(|v| v.pattern == "set-without-local"));
}

#[test]
fn offset_pagination_advisory() {
    let kw = k(&["SE", "LECT id FROM t LIMIT 10 OFFSET $1"]);
    let src = ["use sqlx; let q = \"", &kw, "\";"].concat();
    let r = detect("src/db/page.rs", &src);
    assert!(r.iter().any(|v| v.pattern == "offset-pagination"));
}

#[test]
fn cassandra_allow_filtering_blocked() {
    let kw = k(&["SE", "LECT * FROM events WHERE day = ? ALLOW FILTERING"]);
    let src = ["use sqlx; let q = \"", &kw, "\";"].concat();
    let r = detect("src/db/cass.rs", &src);
    assert!(r.iter().any(|v| v.pattern == "cassandra-allow-filtering"));
}
