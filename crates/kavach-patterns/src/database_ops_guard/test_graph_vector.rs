//! Tests for Graph and Vector database operation detection.

use crate::database_ops_guard::detect;

#[test]
fn cypher_unbounded_path_blocked() {
    let src = "MATCH (a)-[:KNOWS*]->(b) RETURN b";
    let r = detect("queries/social.cypher", src);
    assert!(r.iter().any(|v| v.pattern == "cypher-unbounded-path"));
}

#[test]
fn cypher_bounded_path_ok() {
    let src = "MATCH (a)-[:KNOWS*1..3]->(b) RETURN b";
    let r = detect("queries/social.cypher", src);
    assert!(!r.iter().any(|v| v.pattern == "cypher-unbounded-path"));
}

#[test]
fn vector_query_no_tenant_blocked() {
    let src = r"use pinecone; client.query({vector: v, top_k: 10});";
    let r = detect("src/db/vec.rs", src);
    assert!(r.iter().any(|v| v.pattern == "vector-query-no-tenant"));
}

#[test]
fn vector_query_with_namespace_ok() {
    let src = r"use pinecone; client.query({vector: v, namespace: tenant_id});";
    let r = detect("src/db/vec.rs", src);
    assert!(!r.iter().any(|v| v.pattern == "vector-query-no-tenant"));
}

#[test]
fn vector_upsert_no_dim_check() {
    let src = r"use pinecone; client.upsert(vectors: vec![...]);";
    let r = detect("src/db/vec.rs", src);
    assert!(r.iter().any(|v| v.pattern == "vector-upsert-no-dim-check"));
}
