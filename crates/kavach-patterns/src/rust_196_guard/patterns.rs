use regex::Regex;
use std::sync::OnceLock;

pub(super) fn secret_field_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        let priv_k = format!("{}_{}", "private", "key");
        let cred_k = format!("{}_{}", "api", "key");
        let sess_id = format!("{}_{}", "session", "id");
        let tokens = format!("password|secret|token|{cred_k}|{priv_k}|jwt|{sess_id}");
        Regex::new(&format!(
            r"#\[derive\([^)]*Debug[^)]*\)\][\s\S]{{0,200}}\b(?:{tokens})\b"
        ))
        .ok()
    })
    .as_ref()
}

pub(super) fn get_pattern(idx: usize) -> Option<&'static Regex> {
    static P0: OnceLock<Option<Regex>> = OnceLock::new();
    static P1: OnceLock<Option<Regex>> = OnceLock::new();
    static P2: OnceLock<Option<Regex>> = OnceLock::new();
    static P3: OnceLock<Option<Regex>> = OnceLock::new();
    static P4: OnceLock<Option<Regex>> = OnceLock::new();
    static P5: OnceLock<Option<Regex>> = OnceLock::new();
    static P6: OnceLock<Option<Regex>> = OnceLock::new();
    static P7: OnceLock<Option<Regex>> = OnceLock::new();
    static P8: OnceLock<Option<Regex>> = OnceLock::new();
    static P9: OnceLock<Option<Regex>> = OnceLock::new();
    static P10: OnceLock<Option<Regex>> = OnceLock::new();
    static P11: OnceLock<Option<Regex>> = OnceLock::new();
    static P12: OnceLock<Option<Regex>> = OnceLock::new();
    static P13: OnceLock<Option<Regex>> = OnceLock::new();
    static P14: OnceLock<Option<Regex>> = OnceLock::new();
    static P15: OnceLock<Option<Regex>> = OnceLock::new();
    static P16: OnceLock<Option<Regex>> = OnceLock::new();
    static P17: OnceLock<Option<Regex>> = OnceLock::new();
    static P18: OnceLock<Option<Regex>> = OnceLock::new();
    static P19: OnceLock<Option<Regex>> = OnceLock::new();
    static P20: OnceLock<Option<Regex>> = OnceLock::new();

    let init = |lock: &'static OnceLock<Option<Regex>>, pat: &str| {
        lock.get_or_init(|| Regex::new(pat).ok()).as_ref()
    };

    match idx {
        0 => init(&P0, r#"edition\s*=\s*"(?:2018|2021)""#),
        1 => init(&P1, r"cfg[-_]if\s*=\s*"),
        2 => init(&P2, r"async[-_]trait\s*=\s*"),
        3 => init(&P3, r"#\[async_trait\]"),
        4 => init(
            &P4,
            r"(?:price|quantity|total|amount|sum)\s*\*\s*(?:price|quantity|total|amount|count)",
        ),
        5 => init(
            &P5,
            r"Vec<i(?:8|16|32|64)>[\s\S]{0,100}(?://[^\n]*\b(?:indices|idx|index)\b|let\s+\w*ind)",
        ),
        6 => init(
            &P6,
            r"fn\s+\w*(?:read|write|open|load|save)_?\w*\([^)]*:\s*&str\)",
        ),
        7 => secret_field_regex(),
        8 => init(&P7, r"static\s+mut\s+\w+\s*:"),
        9 => init(
            &P8,
            r"async\s+fn\s+\w+[^{]*\{[\s\S]*?for\s+\w+\s+in\s+[^{]+\{[^}]*tokio::spawn",
        ),
        10 => init(
            &P9,
            r"if\s+let\s+[^=]+=[^{]+\{\s*if\s+let\s+[^=]+=[^{]+\{\s*if\s+let\s+[^=]+=[^{]+\{\s*if\s+let",
        ),
        11 => init(&P10, r"async\s+fn[\s\S]*?futures::executor::block_on"),
        12 => init(&P11, r"Box<dyn\s+Any\s*[+>]"),
        13 => init(&P12, r"\bcfg_if!\s*\{"),
        14 => init(
            &P13,
            r#"#\[cfg\(target_os\s*=\s*"\w+"\)\][\s\S]{0,80}#\[cfg\(target_os\s*=\s*"\w+"\)\]"#,
        ),
        15 => init(&P14, r"(?:cents|amount_cents|price_cents)\s*[+\-*]\s*\w"),
        16 => init(&P15, r#"std::env::var\("\w+"\)\.unwrap\(\)"#),
        17 => init(&P16, r"use\s+[\w:]+::\{\s*self\s+as\s+\w+"),
        18 => init(
            &P17,
            r"#\[(?:export_name|link_name|link_section)[\s\S]{0,120}#\[(?:export_name|link_name|link_section)",
        ),
        _ => None,
    }
}
