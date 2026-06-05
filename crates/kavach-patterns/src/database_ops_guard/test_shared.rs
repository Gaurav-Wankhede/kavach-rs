//! Shared tests for database operation detection (cross-store patterns).

use crate::database_ops_guard::{block_count, detect};

fn k(parts: &[&str]) -> String {
    parts.concat()
}

#[test]
fn n_plus_one_blocked() {
    let src = r"use sqlx; for id in ids { let row = pool.fetch_one(id).await?; }";
    let r = detect("src/repository/u.rs", src);
    assert!(r.iter().any(|v| v.pattern == "n-plus-one"));
}

#[test]
fn non_db_file_skipped() {
    let src = "fn main() { println!(\"hello\"); }";
    let r = detect("src/main.rs", src);
    assert!(r.is_empty());
}

#[test]
fn test_file_skipped() {
    let kw = k(&["SE", "LECT"]);
    let src = ["use sqlx; let q = \"", &kw, " * FROM t\";"].concat();
    let r = detect("tests/db_test.rs", &src);
    assert!(r.is_empty());
}

#[test]
fn cf_d1_string_concat_blocked() {
    let src = r#"export default { async fetch(req, env) { const id = "1"; await env.DB.prepare(`SE` + `LECT id FROM t WHERE id = ${id}`).all(); } }"#;
    let r = detect("worker/src/db.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-d1-string-concat"));
}

#[test]
fn cf_do_fetch_not_rpc_advisory() {
    let src = r#"export default { async fetch(req, env) { const id = env.NS.idFromName("room1"); const stub = env.NS.get(id); await stub.fetch("https://do/handler"); } }"#;
    let r = detect("worker/src/router.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-do-fetch-not-rpc"));
}

#[test]
fn cf_do_block_around_io_blocked() {
    let src = r#"export class Counter extends DurableObject { async inc() { await this.state.blockConcurrencyWhile(async () => { const r = await fetch("https://api.example.com"); }); } }"#;
    let r = detect("worker/src/do.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-do-block-around-io"));
}

#[test]
fn cf_queues_no_idempotency_advisory() {
    let src = r#"export default { async queue(batch, env) { for (const m of batch.messages) { await env.DB.prepare("INSE" + "RT INTO t VALUES (?)").bind(m.body).run(); } } }"#;
    let r = detect("worker/src/consumer.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-queues-no-idempotency"));
}

#[test]
fn block_count_works() {
    let kw = k(&["DR", "OP TABLE x"]);
    let src = ["-- m\n", &kw, ";"].concat();
    assert!(block_count("migrations/001.sql", &src) >= 1);
}
