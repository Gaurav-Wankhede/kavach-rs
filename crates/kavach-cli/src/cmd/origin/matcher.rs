//! Per-line declaration matcher: classify where `name` is DECLARED, config-first.

use super::site::{Kind, Site};
use regex::Regex;

/// All declaration sites of `name` in `content` (one file). Usages are ignored.
pub(super) fn sites_in(name: &str, file: &str, content: &str) -> Vec<Site> {
    let n = regex::escape(name);
    let rules: [(Kind, Regex); 20] = [
        (Kind::EnvVar, re(&format!(r#"(?:env::var\w*|os\.environ(?:\.get)?|getenv)\s*\(?\s*["']{n}["']|process\.env(?:\.{n}|\[\s*["']{n}["']\s*\])"#))),
        (
            Kind::Schema,
            re(&format!(r#"(?i)\b(?:CREATE\s+(?:OR\s+REPLACE\s+)?(?:TABLE|VIEW|INDEX|FUNCTION|PROCEDURE|TYPE)|DEFINE\s+(?:TABLE|FIELD|INDEX)|model|message|service)\s+(?:IF\s+NOT\s+EXISTS\s+)?["'`]?{n}["'`]?\b"#)),
        ),
        (
            Kind::ConfigField,
            re(&format!(r#"^\s*(?:pub\s+)?["']?{n}["']?\s*[:=]\s*[A-Za-z0-9"'{{\[<]"#)),
        ),
        (Kind::Const, re(&format!(r"\b(?:const|val|final)\s+(?:mut\s+)?{n}\b"))),
        (Kind::Static, re(&format!(r"\bstatic\s+(?:mut\s+)?{n}\b"))),
        (Kind::Default, re(&format!(r"impl\s+Default\s+for\s+{n}\b"))),
        (
            Kind::Type,
            re(&format!(r"\b(?:struct|enum|trait|type|union|class|interface)\s+{n}\b")),
        ),
        (
            Kind::Function,
            re(&format!(r"\b(?:fn|func|function|def)\s+{n}\s*[(<]")),
        ),
        (
            Kind::Function,
            re(&format!(r"\b(?:export\s+)?(?:const|let|var)\s+{n}\s*=\s*(?:async\s*)?\([^)]*\)\s*=>")),
        ),
        (
            Kind::DocAnchor,
            re(&format!(r"^\s*#{{1,6}}\s+.*?\b{n}\b")),
        ),
        (Kind::Param, re(&format!(r"\b(?:fn|func|function|def)\s+\w+\s*\([^)]*\b{n}\s*:"))),
        (Kind::Variant, re(&format!(r"^\s*{n}\s*(?:,|\(|\{{|=|$)"))),
        (
            Kind::Variant,
            re(&format!(r"\benum\s+\w+[^;]*[{{,]\s*{n}\s*(?:,|\(|\}}|=)")),
        ),
        (
            Kind::LetBinding,
            re(&format!(r"\b(?:let|var)\s+(?:mut\s+)?{n}\s*[:=]")),
        ),
        (Kind::ConfigField, re(&format!(r#"^\s*["']?{n}["']?\s*[:=]"#))),
        (Kind::Schema, re(&format!(r"(?i)\bTABLE\s+{n}\b"))),
        (Kind::Type, re(&format!(r"\btype\s+{n}\s*="))),
        (Kind::DocAnchor, re(&format!(r"^\s*\[{n}\]:"))),
        (Kind::EnvVar, re(&format!(r"^{n}\s*="))),
        (Kind::Function, re(&format!(r"\b{n}\s*\([^)]*\)\s*\{{"))),
    ];

    let is_md = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"));
    if is_md {
        let md_sites = parse_markdown_sites(name, file, content);
        if !md_sites.is_empty() {
            return md_sites;
        }
    }

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

fn parse_markdown_sites(name: &str, file: &str, content: &str) -> Vec<Site> {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut sites = Vec::new();
    let mut in_heading = false;
    let mut heading_text = String::new();

    let parser = Parser::new(content);
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                if heading_text.split_whitespace().any(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') == name) {
                    let mut current_offset = 0usize;
                    let mut line = 1usize;
                    for l in content.lines() {
                        if current_offset >= range.start {
                            break;
                        }
                        current_offset = current_offset.saturating_add(l.len()).saturating_add(1);
                        line = line.saturating_add(1);
                    }
                    sites.push(Site {
                        kind: Kind::DocAnchor,
                        file: file.to_owned(),
                        line,
                    });
                }
            }
            Event::Text(t) if in_heading => {
                heading_text.push_str(&t);
            }
            _ => {}
        }
    }
    sites
}

fn re(pat: &str) -> Regex {
    // SOURCE: rust-lang.github.io/rust-clippy/master/index.html#expect_used
    Regex::new(pat).unwrap_or_else(|_| {
        #[expect(
            clippy::unwrap_used,
            reason = "literal '$.^' is a compile-time-constant valid regex"
        )]
        Regex::new("$.^").unwrap()
    })
}

#[cfg(test)]
#[path = "matcher_test.rs"]
mod matcher_test;
