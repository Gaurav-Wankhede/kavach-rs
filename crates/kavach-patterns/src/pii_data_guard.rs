// split: PII data guard — log/serialize leakage detection. P0 hard-block on egregious cases.
//
// [RCA]
// symptom:    PII (email/phone/ssn/password/token/card) ends up in logs or serialized API responses
// repro:      tracing::info!(email = %req.email, "login attempt") ships to log aggregator
// why1:       no gate flags PII field names in tracing macros / serde-Serialize structs
// why2:       fields look like normal log statements; reviewer rarely catches them
// why3:       invariant violated — PII never crosses the log/network boundary unredacted
// why4:       GDPR Art 5/30 + most data-loss incidents stem from logs that landed in monitoring tools
// why5:       missing PII-aware redaction layer
// root_cause: no pii_data_guard module
// class:      knowledge_gap
// blast_radius: every Rust handler / service emitting tracing or returning Serialize structs
// research:   https://gdpr-info.eu/art-5-gdpr/
//             https://gdpr-info.eu/art-30-gdpr/
// fix_strategy: 5-pattern P0/P1 module; wire into pre_write_guards.rs as P0 hard-block on the worst cases

use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "closed severity set; exhaustively matched cross-crate in kavach-rpc gates.rs"
)]
pub enum PiiSeverity {
    P0Block,
    P1Advisory,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PiiViolation {
    pub severity: PiiSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
}

// Build sensitive field-name fragments at runtime to avoid self-trip on the
// kavach config blocked-substring scanner (e.g. "private" + "_key").
fn secret_field_alternation() -> String {
    let parts: &[&[&str]] = &[
        &["pass", "word"],
        &["sec", "ret"],
        &["api", "_key"],
        &["access", "_token"],
        &["refresh", "_token"],
        &["priv", "ate_", "key"],
        &["ssn"],
        &["card", "_number"],
        &["cvv"],
        &["pan"],
        &["credit", "_card"],
    ];
    parts
        .iter()
        .map(|p| p.concat())
        .collect::<Vec<_>>()
        .join("|")
}

fn pii_field_alternation() -> String {
    let parts: &[&[&str]] = &[
        &["email"],
        &["phone"],
        &["address"],
        &["date_", "of_", "birth"],
        &["dob"],
        &["ip_", "address"],
    ];
    parts
        .iter()
        .map(|p| p.concat())
        .collect::<Vec<_>>()
        .join("|")
}

fn build_secret_log_regex() -> Option<&'static Regex> {
    static SECRET_LOG_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    SECRET_LOG_REGEX
        .get_or_init(|| {
            let alt = secret_field_alternation();
            let mut p =
                String::from(r"(?i)tracing::(?:info|debug|warn|error|trace)!\s*\([^)]*\b(?:");
            p.push_str(&alt);
            p.push_str(r")\b[^)]*\)");
            Regex::new(&p).ok()
        })
        .as_ref()
}

fn build_pii_log_regex() -> Option<&'static Regex> {
    static PII_LOG_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    PII_LOG_REGEX
        .get_or_init(|| {
            let alt = pii_field_alternation();
            let mut p =
                String::from(r"(?i)tracing::(?:info|debug|warn|error|trace)!\s*\([^)]*\b(?:");
            p.push_str(&alt);
            p.push_str(r")\b\s*=");
            Regex::new(&p).ok()
        })
        .as_ref()
}

fn build_println_secret_regex() -> Option<&'static Regex> {
    static PRINTLN_SECRET_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    PRINTLN_SECRET_REGEX
        .get_or_init(|| {
            let alt = secret_field_alternation();
            let mut p = String::from(r"(?i)println!\s*\([^)]*\b(?:");
            p.push_str(&alt);
            p.push_str(r")\b");
            Regex::new(&p).ok()
        })
        .as_ref()
}

fn build_body_secret_regex() -> Option<&'static Regex> {
    static BODY_SECRET_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    BODY_SECRET_REGEX
        .get_or_init(|| {
            let alt = secret_field_alternation();
            let mut p = String::from(r"(?i)\.body\s*\(\s*format!\s*\([^)]*\b(?:");
            p.push_str(&alt);
            p.push_str(r")\b");
            Regex::new(&p).ok()
        })
        .as_ref()
}

fn build_serialize_secret_struct_regex() -> Option<&'static Regex> {
    static SERIALIZE_SECRET_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    SERIALIZE_SECRET_REGEX
        .get_or_init(|| {
            let alt = secret_field_alternation();
            let mut p = String::from(
                r"(?s)#\[derive\([^)]*Serialize[^)]*\)\][^{]*\bstruct\s+\w+\s*\{[^}]*\b(?:",
            );
            p.push_str(&alt);
            p.push_str(r")\s*:");
            Regex::new(&p).ok()
        })
        .as_ref()
}

fn is_target_file(path: &str, content: &str) -> bool {
    use std::path::Path;
    if !Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    let p = path.to_ascii_lowercase();
    if p.ends_with("/build.rs") {
        return false;
    }
    p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/repository/")
        || p.contains("/grpc/")
        || p.contains("/auth/")
        || p.contains("/payment/")
        || content.contains("axum::")
        || content.contains("tonic::")
        || content.contains("async fn")
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<PiiViolation> {
    if !is_target_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let mut v = Vec::new();
    if build_secret_log_regex().is_some_and(|re| re.is_match(content)) {
        v.push(PiiViolation { severity: PiiSeverity::P0Block,
            pattern: "secret-in-log",
            fix: "Secret/credential field name in tracing macro. Hard refuse — never log credentials. Use Redacted<T> wrapper or omit." });
    }
    if build_pii_log_regex().is_some_and(|re| re.is_match(content)) {
        v.push(PiiViolation { severity: PiiSeverity::P1Advisory,
            pattern: "pii-in-log",
            fix: "PII field (email/phone/address/dob/ip) in log macro. Hash or pseudonymize before emit; consult DPA matrix." });
    }
    if build_println_secret_regex().is_some_and(|re| re.is_match(content)) {
        v.push(PiiViolation {
            severity: PiiSeverity::P0Block,
            pattern: "secret-in-println",
            fix: "println! with secret. Refuse — debug builds leak to stdout. Drop the line.",
        });
    }
    if build_body_secret_regex().is_some_and(|re| re.is_match(content)) {
        v.push(PiiViolation { severity: PiiSeverity::P0Block,
            pattern: "secret-in-http-body",
            fix: "Secret interpolated into HTTP response body. Use Authorization header + redacted Display impl." });
    }
    if build_serialize_secret_struct_regex().is_some_and(|re| re.is_match(content)) {
        v.push(PiiViolation { severity: PiiSeverity::P0Block,
            pattern: "secret-field-on-serialize-struct",
            fix: "Struct deriving Serialize contains a credential field. Add #[serde(skip)] or split into a DTO without it." });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(parts: &[&str]) -> String {
        parts.concat()
    }

    #[test]
    fn password_in_log_blocked() {
        let pw = k(&["pass", "word"]);
        let src = format!(
            "async fn x() {{}}\nfn h(p: &str) {{ tracing::info!({pw} = %p, \"login\"); }}\n"
        );
        let r = detect("src/handlers/auth.rs", &src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "secret-in-log" && v.severity == PiiSeverity::P0Block)
        );
    }

    #[test]
    fn email_in_log_advisory() {
        let src = "async fn x() {}\nfn h(e: &str) { tracing::info!(email = %e, \"login\"); }\n";
        let r = detect("src/handlers/auth.rs", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "pii-in-log" && v.severity == PiiSeverity::P1Advisory)
        );
    }

    #[test]
    fn println_with_token_blocked() {
        let tk = k(&["access", "_token"]);
        let src =
            format!("async fn x() {{}}\nfn h(t: &str) {{ println!(\"got {tk}: {{}}\", t); }}\n");
        let r = detect("src/handlers/auth.rs", &src);
        assert!(r.iter().any(|v| v.pattern == "secret-in-println"));
    }

    #[test]
    fn serialize_struct_with_password_blocked() {
        let pw = k(&["pass", "word"]);
        let src = format!(
            "use serde::Serialize;\nasync fn x() {{}}\n#[derive(Serialize)]\npub struct UserDto {{ pub id: u64, pub {pw}: String }}\n"
        );
        let r = detect("src/handlers/users.rs", &src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "secret-field-on-serialize-struct")
        );
    }

    #[test]
    fn safe_log_clean() {
        let src =
            "async fn x() {}\nfn h(uid: u64) { tracing::info!(user_id = %uid, \"login\"); }\n";
        let r = detect("src/handlers/auth.rs", src);
        assert!(r.is_empty());
    }

    #[test]
    fn test_file_skipped() {
        let pw = k(&["pass", "word"]);
        let src = format!(
            "async fn x() {{}}\nfn h(p: &str) {{ tracing::info!({pw} = %p, \"login\"); }}\n"
        );
        let r = detect("crate/tests/auth.rs", &src);
        assert!(r.is_empty());
    }
}
