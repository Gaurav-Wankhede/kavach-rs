//! Cloudflare Durable Object, R2, and compute-heavy detection.

use super::pattern_set::hit;
use super::types::{DbOpsSeverity, DbOpsViolation, Store};

pub(super) fn detect(content: &str, violations: &mut Vec<DbOpsViolation>) {
    if hit(20, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Cloudflare,
            pattern: "cf-r2-arraybuffer",
            fix: "R2.get().arrayBuffer() loads full object into Worker memory (128 MiB cap). Stream body or use presigned URL.",
            line: 0,
        });
    }
    if hit(21, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Cloudflare,
            pattern: "cf-do-block-around-io",
            fix: "blockConcurrencyWhile around fetch/KV/R2 = lock-across-IO. Reserve for constructor migrations + local storage init.",
            line: 0,
        });
    }
    if hit(22, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Cloudflare,
            pattern: "cf-do-fetch-not-rpc",
            fix: "Use Durable Object RPC methods (compatibility_date >= 2024-04-03) instead of stub.fetch(); typed and ergonomic.",
            line: 0,
        });
    }
    if hit(23, content)
        && !content.contains("dead_letter_queue")
        && !content.contains("idempotency")
        && !content.contains("ack()")
    {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Cloudflare,
            pattern: "cf-queues-no-idempotency",
            fix: "Queues = at-least-once delivery. Add idempotency key check + dead_letter_queue in wrangler.toml.",
            line: 0,
        });
    }
    if hit(24, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Cloudflare,
            pattern: "cf-hyperdrive-rest",
            fix: "Hyperdrive must be used via binding driver (postgres/mysql client + env.HYPERDRIVE.connectionString), not HTTP.",
            line: 0,
        });
    }
    if hit(25, content) && !content.contains("topK:") && !content.contains("topK :") {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Cloudflare,
            pattern: "cf-vectorize-no-topk",
            fix: "Vectorize.query without explicit topK = potentially unbounded read cost. Set topK to expected ceiling.",
            line: 0,
        });
    }
}
