//! Helper functions for database file classification and store detection.

use super::types::Store;

/// Case-insensitive file-extension check.
pub(super) fn has_ext(path: &str, ext: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Check if this is a Cloudflare Worker file.
pub(super) fn is_cloudflare_worker(path: &str, content: &str) -> bool {
    let p = path.to_ascii_lowercase();
    let is_worker_file =
        has_ext(&p, "ts") || has_ext(&p, "js") || has_ext(&p, "mts") || has_ext(&p, "mjs");
    if !is_worker_file {
        return false;
    }
    content.contains(".prepare(")
        || content.contains("DurableObject")
        || content.contains("blockConcurrencyWhile")
        || content.contains("@cloudflare/workers")
        || content.contains("env.KV")
        || content.contains("env.R2")
        || content.contains("env.DB")
        || content.contains(".idFromName(")
        || content.contains("VECTORIZE")
        || content.contains("Hyperdrive")
        || content.contains("hyperdrive")
        || (content.contains("env.") && content.contains(".put("))
}

/// Check if a file uses database-related code.
pub(super) fn is_db_file(path: &str, content: &str) -> bool {
    let p = path.to_ascii_lowercase();
    if has_ext(&p, "sql") || has_ext(&p, "cypher") {
        return true;
    }
    if p.contains("/repository/")
        || p.contains("/repo/")
        || p.contains("/dao/")
        || p.contains("/db/")
        || p.contains("/database/")
        || p.contains("/migrations/")
        || p.contains("/queries/")
    {
        return true;
    }
    let c = content;
    c.contains("sqlx::")
        || c.contains("diesel::")
        || c.contains("sea_orm")
        || c.contains("mongodb::")
        || c.contains("redis::")
        || c.contains("aws_sdk_dynamodb")
        || c.contains("neo4rs")
        || c.contains("pinecone")
        || c.contains("qdrant")
        || c.contains("pgvector")
        || is_cloudflare_worker(path, c)
}

/// Classify the database store type.
pub(super) fn classify_store(path: &str, content: &str) -> Store {
    let p = path.to_ascii_lowercase();
    if is_cloudflare_worker(path, content) {
        return Store::Cloudflare;
    }
    if has_ext(&p, "cypher") || content.contains("neo4rs") || content.contains("MATCH (") {
        return Store::Graph;
    }
    if content.contains("pinecone")
        || content.contains("qdrant")
        || content.contains("pgvector")
        || content.contains("VECTOR(")
    {
        return Store::Vector;
    }
    if content.contains("redis::") || content.contains("aws_sdk_dynamodb") {
        return Store::Kv;
    }
    if content.contains("mongodb::")
        || content.contains("$where")
        || content.contains("writeConcern")
    {
        return Store::NoSql;
    }
    if content.contains("sqlx::")
        || content.contains("diesel::")
        || content.contains("sea_orm")
        || has_ext(&p, "sql")
    {
        return Store::Sql;
    }
    Store::Unknown
}
