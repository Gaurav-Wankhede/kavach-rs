//! Database anti-patterns.

use super::types::{Severity, mk};
use crate::config::j;

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    let sel = j(&["SE", "LE", "CT"]);
    let del = j(&["DE", "LE", "TE"]);
    let upd = j(&["UP", "DA", "TE"]);
    let trn = j(&["TR", "UN", "CA", "TE"]);
    let drp = j(&["DR", "OP"]);
    let frm = j(&["FR", "OM"]);
    let tbl = j(&["TA", "BLE"]);
    let idx = j(&["IN", "DEX"]);
    let dbs = j(&["DA", "TA", "BA", "SE"]);

    vec![
        (
            mk(&format!(r"{sel}\s+\*\s+{frm}")),
            "SELECT_STAR",
            "star select — name columns explicitly",
            Severity::P2Medium,
        ),
        (
            mk(&format!(r"(?i){del}\s+{frm}\s+\w+(?:\s*;|\s*$)")),
            "DELETE_NO_WHERE",
            "delete without where — add condition",
            Severity::P0Critical,
        ),
        (
            mk(&format!(r"(?i){upd}\s+\w+\s+SET\s+")),
            "CHECK_UPDATE_WHERE",
            "update statement — verify WHERE clause exists",
            Severity::P1High,
        ),
        (
            mk(&format!(r"(?i){trn}\s+{tbl}")),
            "TRUNCATE_APP",
            "truncate in app — use migrations only",
            Severity::P0Critical,
        ),
        (
            mk(&format!(r"(?i){drp}\s+(?:{tbl}|{idx}|{dbs})")),
            "DROP_APP",
            "drop in app — use migrations only",
            Severity::P0Critical,
        ),
        (
            mk(&format!(r"(?i){sel}\s+[^;]*{frm}\s+\w+")),
            "CHECK_LIMIT",
            "select query — verify LIMIT clause for pagination",
            Severity::P2Medium,
        ),
        (
            mk(r"(?i)OFFSET\s+\$?\d+"),
            "OFFSET_PAGINATION",
            "offset pagination — use keyset cursor",
            Severity::P2Medium,
        ),
        (
            mk(r"(?:postgres|mysql|mongodb)://\w+:[^@]+@"),
            "HARDCODED_CREDS",
            "Hardcoded DB credentials — use env var",
            Severity::P0Critical,
        ),
        (
            mk(r"(?s)for\s+\w+\s+in\s+\w+\s*\{[^}]*sqlx::query"),
            "N_PLUS_ONE",
            "Query in loop — use JOIN or batch query",
            Severity::P0Critical,
        ),
        (
            mk(r"(?s)(?:insert|update|delete).*\.await\?;.*(?:insert|update|delete).*\.await\?"),
            "NO_TRANSACTION",
            "Multiple writes without transaction — wrap in tx",
            Severity::P1High,
        ),
    ]
}
