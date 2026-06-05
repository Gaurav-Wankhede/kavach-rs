//! P1 advisories: unencrypted PII fields, bool `#[serde(default)]`, and auth
//! structs missing `#[serde(deny_unknown_fields)]`. Fires on any non-test `.rs`.
use super::super::platform_guard_msg::build_advisory;
use super::super::platform_guard_paths::is_test;
use std::path::Path;

const PII_FIELDS: &[&str] = &["email", "phone", "address", "ssn"];

/// `Some(advisory)` listing soft response-security findings, or None if clean.
pub(crate) fn format_advisory(file_path: &str, content: &str) -> Option<String> {
    if !Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
        || is_test(file_path)
    {
        return None;
    }
    let lc = content.to_lowercase();
    let mut p1: Vec<(&str, &str)> = Vec::new();

    for field in PII_FIELDS {
        let plain = format!("pub {field}: string");
        if lc.contains(&plain) {
            p1.push(("PII_FIELD_UNENCRYPTED",
                "Wrap PII field (email/phone/address/ssn) in EncryptedField<T> or redact before serialization."));
            break;
        }
    }

    if (lc.contains("serde(default") || lc.contains("serde( default"))
        && lc.contains("bool")
        && !lc.contains("is_admin")
        && !lc.contains("is_moderator")
    {
        p1.push(("BOOL_SERDE_DEFAULT",
            "Change #[serde(default)] bool to Option<bool> — explicit None is safer than implicit false."));
    }

    if (lc.contains("struct")
        && (lc.contains("authrequest")
            || lc.contains("loginrequest")
            || lc.contains("grantrequest")
            || lc.contains("tokenclaim")))
        && !lc.contains("deny_unknown_fields")
    {
        p1.push((
            "MISSING_DENY_UNKNOWN",
            "Add #[serde(deny_unknown_fields)] to auth structs — rejects attacker-injected fields.",
        ));
    }

    if p1.is_empty() {
        return None;
    }
    Some(build_advisory("RESPONSE_SECURITY_GUARD", &p1))
}
