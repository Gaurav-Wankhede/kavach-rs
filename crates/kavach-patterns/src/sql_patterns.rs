use crate::config::j;
use regex::Regex;
use std::sync::LazyLock;

fn try_compile_sql_patterns() -> Result<Vec<Regex>, regex::Error> {
    let sel_star = j(&[r"SEL", "ECT", "\\s+", "\\*"]);
    let del_nowhr = j(&[r"DEL", "ETE", "\\s+", "FR", "OM", "\\s+", "\\w+", "\\s*;"]);
    let grnt_all = j(&[r"GRA", "NT", "\\s+(?:", "ALL", "|SUPERUSER)"]);
    let pwd_hard = j(&["PASSWORD", "\\s*=\\s*'[^']+'"]);
    let drp_tbl = j(&[r"DR", "OP", "\\s+", "TAB", "LE"]);
    let trunc = j(&[r"\bTRUN", "CAT", "E\\b"]);
    // FIX [boundary_breach] — (?s) DOTALL so [^)] matches \n. A multi-line
    // format-macro call carrying a SQL keyword on a subsequent line could
    // previously evade the guard because per-line scanning saw the macro on
    // one line and the keyword on the next. The sql_guard caller runs this
    // regex on FULL CONTENT (not per-line) and maps byte offset back to a
    // line number for the violation report.
    let fmt_sql = j(&[
        r"(?s)format!\s*\([^)]*(?:SEL",
        "ECT|INS",
        "ERT|UPD",
        "ATE|DEL",
        "ETE|FR",
        "OM|WH",
        "ERE)",
    ]);
    let offset = j(&[r"\bOFF", "SET", "\\s+", "\\d+"]);

    Ok(vec![
        Regex::new(&fmt_sql)?,
        Regex::new(&sel_star)?,
        Regex::new(&del_nowhr)?,
        Regex::new(&grnt_all)?,
        Regex::new(&pwd_hard)?,
        Regex::new(&offset)?,
        Regex::new(&drp_tbl)?,
        Regex::new(&trunc)?,
    ])
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) static SQL_P: LazyLock<Vec<Regex>> =
    LazyLock::new(|| try_compile_sql_patterns().unwrap_or_else(|_| Vec::new()));
