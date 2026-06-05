//! Cloudflare KV and D1 detection.

use super::pattern_set::{D1_SELECT_STAR, hit};
use super::types::{DbOpsSeverity, DbOpsViolation, Store};

pub(super) fn detect(content: &str, violations: &mut Vec<DbOpsViolation>) {
    if hit(16, content) && !content.contains("expirationTtl") && !content.contains("expiration:") {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Cloudflare,
            pattern: "cf-kv-no-ttl",
            fix: "KV.put without expirationTtl = unbounded growth + no eviction. Add expirationTtl in seconds.",
            line: 0,
        });
    }
    if hit(17, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Cloudflare,
            pattern: "cf-kv-write-in-loop",
            fix: "KV is limited to 1 write/sec per key. Batch into Queue or use Durable Object SQLite.",
            line: 0,
        });
    }
    if hit(18, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Cloudflare,
            pattern: "cf-d1-string-concat",
            fix: "SQL injection via template/concat in .prepare(). Use .prepare with ? placeholder + .bind(value).",
            line: 0,
        });
    }
    if D1_SELECT_STAR
        .as_ref()
        .is_some_and(|re| re.is_match(content))
    {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Cloudflare,
            pattern: "cf-d1-select-star",
            fix: "D1 bills per row read. Naming columns reduces read units vs star projection.",
            line: 0,
        });
    }
}
