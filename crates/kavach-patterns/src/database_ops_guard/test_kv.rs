//! Tests for Key-Value store-specific database operation detection.

use crate::database_ops_guard::detect;

#[test]
fn redis_keys_blocked() {
    let src = r#"use redis::Commands; con.cmd("KEYS *");"#;
    let r = detect("src/db/cache.rs", src);
    assert!(r.iter().any(|v| v.pattern == "redis-keys-glob"));
}

#[test]
fn redis_set_no_ttl_advisory() {
    let src = r#"use redis::Commands; con.set("user:1", &val);"#;
    let r = detect("src/db/cache.rs", src);
    assert!(r.iter().any(|v| v.pattern == "redis-set-no-ttl"));
}

#[test]
fn redis_set_with_ttl_ok() {
    let src = r#"use redis::Commands; con.set_ex("user:1", &val, 3600);"#;
    let r = detect("src/db/cache.rs", src);
    assert!(!r.iter().any(|v| v.pattern == "redis-set-no-ttl"));
}

#[test]
fn dynamodb_scan_blocked() {
    let src = r"use aws_sdk_dynamodb; client.scan().send().await?;";
    let r = detect("src/db/ddb.rs", src);
    assert!(r.iter().any(|v| v.pattern == "dynamodb-scan"));
}
