//! Data validation anti-patterns.

use super::types::{Severity, mk};

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    vec![
        (
            mk(r"(?s)(?:Json|Query|Path|Form)<[^>]+>\s*\)\s*(?:->.*?)?\{[^}]{0,50}\}"),
            "NO_VALIDATE",
            "Handler takes input without validation — add Validate derive",
            Severity::P1High,
        ),
        (
            mk(r"pub\s+\w+:\s*String(?:\s*,|\s*\})"),
            "UNBOUNDED_STRING",
            "String without length limit — add #[validate(length)]",
            Severity::P2Medium,
        ),
        (
            mk(r"(?:email|e_mail):\s*String"),
            "EMAIL_STRING",
            "Email as String — use #[validate(email)]",
            Severity::P1High,
        ),
        (
            mk(r"(?:url|uri|link|href):\s*String"),
            "URL_STRING",
            "URL as String — use url::Url type",
            Severity::P1High,
        ),
        (
            mk(r"(?:phone|mobile|tel):\s*String"),
            "PHONE_STRING",
            "Phone as String — use phonenumber crate",
            Severity::P2Medium,
        ),
        (
            mk(r"(?:user_id|account_id|order_id):\s*String"),
            "ID_STRING",
            "ID as String — use typed newtype or Uuid",
            Severity::P2Medium,
        ),
        (
            mk(r"#\[serde\(default\)\]\s*\n?\s*pub\s+\w+:\s*bool"),
            "SERDE_DEFAULT_BOOL",
            "serde(default) on bool — use Option<bool>",
            Severity::P0Critical,
        ),
        (
            mk(r"pub\s+\w+:\s*Vec<[A-Z]\w+>(?:\s*,|\s*\})"),
            "NO_NESTED_VALIDATE",
            "Nested Vec<T> — add #[validate(nested)]",
            Severity::P2Medium,
        ),
        (
            mk(r"pub\s+(?:age|qty|count|amount):\s*(?:i32|i64|u32|u64)"),
            "NO_RANGE",
            "Numeric field — add #[validate(range)]",
            Severity::P2Medium,
        ),
        (
            mk(r"(?:password|passwd|pwd):\s*String"),
            "WEAK_PASSWORD",
            "Password as String — add min length + complexity validation",
            Severity::P1High,
        ),
    ]
}
