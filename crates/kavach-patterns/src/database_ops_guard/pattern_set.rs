//! Compiled regex pattern set for database operation detection.

use super::regex_builders::{
    build_d1_select_star_regex, build_destructive_sql_regex, build_format_sql_regex,
    build_select_star_regex,
};
use regex::Regex;
use std::sync::LazyLock;

pub(super) static SELECT_STAR: LazyLock<Option<Regex>> = LazyLock::new(build_select_star_regex);
pub(super) static FORMAT_SQL: LazyLock<Option<Regex>> = LazyLock::new(build_format_sql_regex);
pub(super) static DESTRUCTIVE_SQL: LazyLock<Option<Regex>> =
    LazyLock::new(build_destructive_sql_regex);
pub(super) static D1_SELECT_STAR: LazyLock<Option<Regex>> =
    LazyLock::new(build_d1_select_star_regex);

#[expect(
    clippy::trivial_regex,
    reason = "a few literal entries stay Regex so the whole set shares one matcher path"
)]
pub(super) static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        Regex::new(r"\.begin\(\)\.await").expect("static regex"),
        Regex::new(r"(?is)\bUPDATE\s+\w+\s+SET\b").expect("static regex"),
        Regex::new(r#"(?i)"SET\s+[A-Za-z_.]+\s*="#).expect("static regex"),
        Regex::new(r"(?i)\bOFFSET\s+\$?\d").expect("static regex"),
        Regex::new(r"\.find\([^)]*\)(?:\.toArray\(\)|\.to_vec\(\))").expect("static regex"),
        Regex::new(r#"\$where[`'"]?\s*:\s*[`'"]"#).expect("static regex"),
        Regex::new(r"\.(insertOne|insertMany|updateOne|updateMany|deleteOne|deleteMany)\(").expect("static regex"),
        Regex::new(r#"\.set\(\s*[`'"][^`'"]+[`'"]\s*,\s*[^,)]+\)"#).expect("static regex"),
        Regex::new(r#"\bKEYS\s+[`'"]?\*"#).expect("static regex"),
        Regex::new(r"\.scan\(\)|ScanInput|scan_request").expect("static regex"),
        Regex::new(r"\[\s*[:\w]*\*\s*\]").expect("static regex"),
        Regex::new(r"\.upsert\(\s*vec(?:tor)?s?\s*[:=]").expect("static regex"),
        Regex::new(r"\.query\(\s*\{?\s*vector\s*[:=]").expect("static regex"),
        Regex::new(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,500}\.(?:fetch_one|fetch_all|fetch_optional|find_one|get|query)\b").expect("static regex"),
        Regex::new(r"\.acquire\(\)\.await\?").expect("static regex"),
        Regex::new(r"(?i)\bALLOW\s+FILTERING\b").expect("static regex"),
        Regex::new(r"\.(?:KV|kv|[A-Z_]+_KV|SESSION|CACHE|CONFIG)\.put\s*\(").expect("static regex"),
        Regex::new(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,500}\.(?:KV|kv|[A-Z_]+_KV)\.put\s*\(").expect("static regex"),
        Regex::new(r#"(?s)\.prepare\s*\([^)]*\$\{|(?s)\.prepare\s*\([^)]*['`"]\s*\+\s*\w"#).expect("static regex"),
        Regex::new(r"a\bcollision_unmatchable\bz").expect("static regex"),
        Regex::new(r"(?s)\.(?:R2|r2|[A-Z_]+_BUCKET|BUCKET)\.get\(.{0,500}\.arrayBuffer\(\)").expect("static regex"),
        Regex::new(r"(?s)blockConcurrencyWhile\s*\(\s*async[^}]*?(?:fetch\(|\.get\(|\.put\(|\.list\()").expect("static regex"),
        Regex::new(r"(?s)\.(?:idFromName|idFromString)\(.{0,500}\.fetch\(").expect("static regex"),
        Regex::new(r"export\s+default\s*\{[^}]*\bqueue\s*\(").expect("static regex"),
        Regex::new(r#"fetch\s*\(\s*[`'"]https?://[^`'"]*hyperdrive"#).expect("static regex"),
        Regex::new(r"\.(?:VECTORIZE|VECTOR_INDEX|[A-Z_]+_INDEX)\.query\s*\(").expect("static regex"),
    ]
    .into_iter()
    .collect()
});

/// Does `PATTERNS[idx]` match `content`?
pub(super) fn hit(idx: usize, content: &str) -> bool {
    PATTERNS.get(idx).is_some_and(|re| re.is_match(content))
}

/// Iterate matches of `PATTERNS[idx]` over `content`.
pub(super) fn matches_of(idx: usize, content: &str) -> Vec<regex::Match<'_>> {
    PATTERNS
        .get(idx)
        .map(|re| re.find_iter(content).collect())
        .unwrap_or_default()
}
