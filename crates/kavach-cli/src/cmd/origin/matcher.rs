//! Per-line declaration matcher: classify where `name` is DECLARED, config-first.

use super::site::{Kind, Site};
use regex::Regex;

/// All declaration sites of `name` in `content` (one file). Usages are ignored.
pub(super) fn sites_in(name: &str, file: &str, content: &str) -> Vec<Site> {
    let n = regex::escape(name);
    let rules: [(Kind, Regex); 11] = [
        (Kind::EnvVar, re(&format!(r#"env::var\w*\(\s*"{n}""#))),
        (Kind::Const, re(&format!(r"\bconst\s+{n}\b"))),
        (Kind::Static, re(&format!(r"\bstatic\s+(?:mut\s+)?{n}\b"))),
        (Kind::Default, re(&format!(r"impl\s+Default\s+for\s+{n}\b"))),
        (Kind::Type, re(&format!(r"\b(?:struct|enum|trait|type|union)\s+{n}\b"))),
        (Kind::Function, re(&format!(r"\bfn\s+{n}\s*[(<]"))),
        (Kind::Param, re(&format!(r"\bfn\s+\w+\s*\([^)]*\b{n}\s*:"))),
        (Kind::Variant, re(&format!(r"^\s*{n}\s*(?:,|\(|\{{|=|$)"))),
        (Kind::Variant, re(&format!(r"\benum\s+\w+[^;]*[{{,]\s*{n}\s*(?:,|\(|\}}|=)"))),
        (Kind::ConfigField, re(&format!(r"^\s*(?:pub\s+)?{n}\s*:\s*[A-Za-z]"))),
        (Kind::LetBinding, re(&format!(r"\blet\s+(?:mut\s+)?{n}\s*[:=]"))),
    ];
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        for (kind, re) in &rules {
            if re.is_match(line) {
                out.push(Site {
                    kind: *kind,
                    file: file.to_owned(),
                    line: i.saturating_add(1),
                });
            }
        }
    }
    out
}

fn re(pat: &str) -> Regex {
    Regex::new(pat).unwrap_or_else(|_| Regex::new("$.^").expect("never-match fallback compiles"))
}

#[cfg(test)]
#[path = "matcher_test.rs"]
mod matcher_test;
