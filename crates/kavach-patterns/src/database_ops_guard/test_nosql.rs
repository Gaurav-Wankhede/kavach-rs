//! Tests for NoSQL-specific database operation detection.

use crate::database_ops_guard::detect;

#[test]
fn mongo_find_no_limit_blocked() {
    let src = r"use mongodb::Collection; coll.find(filter).to_vec().await?;";
    let r = detect("src/repository/m.rs", src);
    assert!(r.iter().any(|v| v.pattern == "mongo-find-no-limit"));
}

#[test]
fn mongo_where_injection_blocked() {
    let src = r#"use mongodb::bson; doc!{"$where": "this.x > 0"};"#;
    let r = detect("src/db/m.rs", src);
    assert!(r.iter().any(|v| v.pattern == "mongo-where-injection"));
}

#[test]
fn mongo_write_with_concern_ok() {
    let src = r"use mongodb::Collection; coll.insert_one(doc, writeConcern).await?;";
    let r = detect("src/db/m.rs", src);
    assert!(!r.iter().any(|v| v.pattern == "mongo-write-no-concern"));
}
