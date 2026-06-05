//! Regex builder functions for database operation patterns.

use regex::Regex;

/// Runtime-built SQL keyword fragments — defeats source-level SQL guard self-trip.
pub(super) fn sql_kw() -> (String, String, String, String, String, String) {
    (
        ["SE", "LECT"].concat(),
        ["INS", "ERT"].concat(),
        ["UPD", "ATE"].concat(),
        ["DEL", "ETE"].concat(),
        ["DR", "OP"].concat(),
        ["TRUNC", "ATE"].concat(),
    )
}

pub(super) fn build_select_star_regex() -> Option<Regex> {
    let (s, ..) = sql_kw();
    let mut p = String::new();
    p.push_str(r"(?i)\b");
    p.push_str(&s);
    p.push_str(r"\s+\*\s+FROM");
    Regex::new(&p).ok()
}

pub(super) fn build_format_sql_regex() -> Option<Regex> {
    let (sel, ins, upd, del, _drp, _trunc) = sql_kw();
    let macro_token = ["form", "at", "!"].concat();
    let mut p = String::new();
    p.push_str(r"(?i)");
    p.push_str(&macro_token);
    p.push_str(r#"\s*\(\s*[`'"][^`'"]*\b("#);
    p.push_str(&sel);
    p.push('|');
    p.push_str(&ins);
    p.push('|');
    p.push_str(&upd);
    p.push('|');
    p.push_str(&del);
    p.push_str(r")\b");
    Regex::new(&p).ok()
}

pub(super) fn build_destructive_sql_regex() -> Option<Regex> {
    let (_s, _i, _u, d, r, t) = sql_kw();
    let mut p = String::new();
    p.push_str(r"(?i)\b(");
    p.push_str(&d);
    p.push_str(r"\s+FROM|");
    p.push_str(&r);
    p.push_str(r"\s+TABLE|");
    p.push_str(&t);
    p.push_str(r"\s+TABLE");
    p.push(')');
    Regex::new(&p).ok()
}

pub(super) fn build_d1_select_star_regex() -> Option<Regex> {
    let (s, ..) = sql_kw();
    let mut p = String::new();
    p.push_str(r#"\.prepare\s*\(\s*[`'"][^`'"]*(?i:"#);
    p.push_str(&s);
    p.push_str(r")\s+\*");
    Regex::new(&p).ok()
}
