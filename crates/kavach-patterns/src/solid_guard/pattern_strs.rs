//! Regex pattern strings and initialization for SOLID gate detection.

use regex::Regex;
use std::sync::OnceLock;

static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

pub(super) const PATTERN_STRS: &[&str] = &[
    r"a\bnever_match_a\bz",
    r"(?s)pub\s+struct\s+\w+\s*\{([^}]+)\}",
    r"(?s)match\s+\w+\s*\{[^}]*(?:Stripe|Paypal|Twilio|Email|Sms|Smtp|S3|R2|Postgres|Mysql|Mongo)\b[^}]*=>",
    r#"(?i)if\s+\w+\s*==\s*[`'"](?:stripe|paypal|twilio|smtp|email|sms|s3|r2)[`'"]"#,
    r"(?s)impl\s+\w+\s+for\s+\w+\s*\{[^}]*?fn\s+\w+\s*\([^)]*\)[^{]*\{[^}]*?(?:panic!|unimplemented!|todo!)\s*\(",
    r"(?s)fn\s+\w+\s*\([^)]*\)\s*->\s*Result<[^>]+>\s*\{[^}]*?\.unwrap\(\)\s*\}",
    r"(?s)pub\s+trait\s+\w+(?::[^{]*)?\s*\{([^}]+)\}",
    r"(?s)pub\s+trait\s+\w+\s*\{[^}]*?fn\s+(?:get|read|find|fetch)[^}]*?fn\s+(?:put|write|insert|save)[^}]*?fn\s+(?:delete|remove|drop)",
    r"pub\s+(?:async\s+)?fn\s+\w+\s*(?:<[^>]*>)?\s*\([^)]*:\s*&?(?:mut\s+)?(?:sqlx::PgPool|sqlx::MySqlPool|sqlx::SqlitePool|reqwest::Client|mongodb::Client|redis::Client)\b",
    r"(?s)pub\s+struct\s+\w*Service\s*\{[^}]*?:\s*(?:sqlx::Pg(?:Pool|Connection)|reqwest::Client|mongodb::Client)\b",
    r"(?s)async\s+fn\s+\w+\s*\([^)]*\)[^{]*\{",
    r"(?s)fn\s+(?:process|charge|notify|send|dispatch)_\w+[^{]*\{[^}]*?match\s+\w+\s*\{[^}]*?(?:Stripe|Paypal|Twilio)",
    r"\bfn\s+(?:do_everything|handle_all|process_all|run_all)\s*\(",
    r"State\s*<\s*&?(?:Arc<)?(?:sqlx::PgPool|sqlx::MySqlPool|sqlx::SqlitePool|reqwest::Client|mongodb::Client|redis::Client)",
    r"(?s)impl(?:<[^{>]*>)?\s+(?:tower::)?Service\s*<[^{]*?>\s+for\s+\w+\s*\{.{0,2000}?(?:panic!|unimplemented!|todo!)\s*\(",
    r"(?s)pub\s+async\s+fn\s+\w+[^{]*\{[^}]*?Router::new\(\)",
    r"Extension\s*<\s*&?(?:Arc<)?(?:sqlx::PgPool|sqlx::MySqlPool|reqwest::Client|mongodb::Client)",
    r"#\[derive\s*\(([^)]+)\)\]\s*(?:#\[[^\]]+\]\s*)*(?:pub\s+)?struct\s+\w+",
    r"(?m)^\s*use\s+crate::(?:infra|infrastructure|persistence|adapters|adapter|driver|drivers)::",
    r"(?s)impl\s+\w+\s+for\s+\w+\s*\{.{0,2000}?(?:tokio::runtime::Handle::block_on|futures::executor::block_on|\bblock_on\s*\()",
    r"(?:pub\s+)?async\s+fn\s+\w+\s*\(\s*\w+\s*:\s*(?:axum::http::|http::|axum::extract::)?Request\b",
    r"(?:lazy_static!\s*\{|Lazy\s*<\s*(?:sqlx::Pg(?:Pool|Connection)|reqwest::Client|mongodb::Client))",
];

pub(super) const CONFLATED_DERIVES: &[&str] = &[
    "FromRow",
    "Serialize",
    "Deserialize",
    "ToSchema",
    "sqlx::Type",
    "sqlx::FromRow",
];

pub(super) fn get_patterns() -> &'static Vec<Regex> {
    PATTERNS.get_or_init(|| {
        let mut pats = Vec::with_capacity(PATTERN_STRS.len());
        for s in PATTERN_STRS {
            if let Ok(r) = Regex::new(s) {
                pats.push(r);
            }
        }
        pats
    })
}
