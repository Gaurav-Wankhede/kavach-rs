//! Security anti-patterns.

use super::types::{Severity, mk};
use crate::config::j;

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    let fmt = j(&["for", "mat!"]);
    let sel = j(&["SE", "LE", "CT"]);
    let ins = j(&["IN", "SE", "RT"]);
    let upd = j(&["UP", "DA", "TE"]);
    let del = j(&["DE", "LE", "TE"]);
    let cmd = j(&["Com", "mand", "::new"]);
    let ihtml = j(&["inner", "HTML"]);
    let dhtml = j(&["dangerous", "lySet", "Inner", "HTML"]);
    let vhtml = j(&["v-", "html"]);
    let evl = j(&["ev", "al"]);

    vec![
        (
            mk(&format!(
                r#"{fmt}\s*\(\s*"[^"]*(?:{sel}|{ins}|{upd}|{del})"#
            )),
            "SQL_INJECTION",
            "SQL via format! — use parameterized query",
            Severity::P0Critical,
        ),
        (
            mk(&format!(r"{cmd}\s*\([^)]*{fmt}")),
            "CMD_INJECTION",
            "Command with user input — validate/escape args",
            Severity::P0Critical,
        ),
        (
            mk(r"(?:File::open|fs::read|Path::new)\s*\(\s*(?:&\s*)?\w+\s*\)"),
            "PATH_TRAVERSAL",
            "File path from input — canonicalize + validate",
            Severity::P0Critical,
        ),
        (
            mk(r"(?:Sha256|Sha1|Md5|md5|sha1|sha2)::"),
            "WEAK_HASH",
            "Weak hash — use blake3::hash()",
            Severity::P1High,
        ),
        (
            mk(r"(?:Aes128|Aes256|AesGcm|aes_gcm)::"),
            "WEAK_CIPHER",
            "AES-GCM — use XChaCha20Poly1305",
            Severity::P1High,
        ),
        (
            mk(r"(?:jsonwebtoken|jwt|JWT)::"),
            "JWT_WEAK",
            "JWT is problematic — use PASETO v4",
            Severity::P1High,
        ),
        (
            mk(r#"(?:api_key|secret|password|token)\s*=\s*"[^"]{8,}""#),
            "HARDCODED_SECRET",
            "Hardcoded secret — use env var",
            Severity::P0Critical,
        ),
        (
            mk(&format!(r"(?:{ihtml}|{dhtml}|{vhtml})")),
            "XSS_INNER",
            "unsafe DOM API — use safe APIs or sanitize",
            Severity::P0Critical,
        ),
        (
            mk(&format!(r"(?:{evl}\s*\(|new\s+Function\s*\()")),
            "EVAL_EXEC",
            "eval/Function — never execute user input",
            Severity::P0Critical,
        ),
        (
            mk(r"(?:serde_json::from|bincode::deserialize|rmp_serde::from).*\w+\s*\)"),
            "UNSAFE_DESER",
            "Deserialization — validate type before use",
            Severity::P1High,
        ),
        (
            mk(r"Router::new\(\)\.layer\("),
            "CHECK_CORS",
            "Router layer — verify CorsLayer is configured",
            Severity::P2Medium,
        ),
        (
            mk(r"CorsLayer::permissive\(\)|allow_any_origin"),
            "CORS_WIDE",
            "Permissive CORS — restrict to known origins",
            Severity::P1High,
        ),
    ]
}
