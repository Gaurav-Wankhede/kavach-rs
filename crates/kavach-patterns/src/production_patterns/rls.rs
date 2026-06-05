//! Row-level security anti-patterns.

use super::types::{Severity, mk};
use crate::config::j;

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    let ssn = j(&["SE", "SS", "ION"]);
    let crt = j(&["CR", "EA", "TE"]);
    let tbl = j(&["TA", "BLE"]);

    vec![
        (
            mk(r"(?i)SET\s+(?:app\.|rls\.)"),
            "CHECK_SET_LOCAL",
            "SET context — verify using SET LOCAL for RLS",
            Severity::P1High,
        ),
        (
            mk(&format!(r"BYPASSRLS|SET\s+{ssn}\s+AUTHORIZATION")),
            "RLS_BYPASS",
            "RLS bypass — never use in app code",
            Severity::P0Critical,
        ),
        (
            mk(&format!(r"(?i){crt}\s+{tbl}\s+\w+")),
            "CHECK_RLS",
            "create table — verify ENABLE ROW LEVEL SECURITY",
            Severity::P1High,
        ),
        (
            mk(r"SECURITY\s+DEFINER"),
            "CHECK_DEFINER",
            "security definer — verify SET LOCAL in function",
            Severity::P1High,
        ),
        (
            mk(r"(?i)FROM\s+(?:pg_|information_schema\.)"),
            "CATALOG_ACCESS",
            "Direct catalog access — use application queries",
            Severity::P2Medium,
        ),
    ]
}
