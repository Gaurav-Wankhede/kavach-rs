//! Walk the tree once, yielding every declaration site enriched into a Candidate.

use super::role_query::Candidate;
use super::secret_hints::is_secret;
use super::{matcher, site::Site, source_files};
use std::path::Path;

#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "line index i+1 for 1-based display; bounded by file line count"
)]
pub(super) fn walk(root: &Path) -> Vec<Candidate> {
    use ast_grep_core::{AstGrep, Language};
    let mut out = Vec::new();
    for path in source_files(root) {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        // SOURCE: rust-lang.github.io/rust-clippy/master/index.html#map_unwrap_or
        let rel_path = path
            .strip_prefix(root)
            .ok()
            .filter(|r| !r.as_os_str().is_empty())
            .map_or_else(|| path.clone(), Path::to_path_buf);
        let rel = rel_path.to_string_lossy().into_owned();

        // 1. Dynamic AST extraction if supported programming language
        if let Some(lang) = ast_grep_language::SupportLang::from_path(&path) {
            let grep = AstGrep::new(&src, lang);
            let lines: Vec<&str> = src.lines().collect();
            for node in grep.root().dfs() {
                if let Some((kind_str, sym)) = crate::cmd::trace::any_definition_node(&node) {
                    let line_no = super::ast_scan::offset_to_line(&src, node.range().start);
                    let line_text = lines.get(line_no.saturating_sub(1)).copied().unwrap_or("");
                    let secret = is_secret(&sym);
                    let value = capture_value(line_text);
                    let mapped_kind = match kind_str.as_str() {
                        "function" => super::site::Kind::Function,
                        "class" | "struct" | "interface" | "type" => super::site::Kind::Type,
                        "enum" => super::site::Kind::Type,
                        "const" => super::site::Kind::Const,
                        "variable" | "assignment" => super::site::Kind::LetBinding,
                        _ => super::site::Kind::Type,
                    };
                    out.push(Candidate {
                        name: sym,
                        kind: mapped_kind,
                        file: rel.clone(),
                        line: line_no,
                        value,
                        is_secret: secret,
                    });
                }
            }
        }

        // 2. Line-by-line fallback for config/env/legacy matches
        for (i, line) in src.lines().enumerate() {
            out.extend(candidates_on_line(line, &rel, i + 1));
        }
    }
    out
}

fn candidates_on_line(line: &str, file: &str, line_no: usize) -> Vec<Candidate> {
    let Some(name) = declared_name(line) else {
        return Vec::new();
    };
    let secret = is_secret(&name);
    let value = capture_value(line);
    matcher::sites_in(&name, file, line)
        .into_iter()
        .map(|s: Site| Candidate {
            name: name.clone(),
            kind: s.kind,
            file: s.file,
            line: line_no.max(s.line),
            value: value.clone(),
            is_secret: secret,
        })
        .collect()
}

#[expect(
    clippy::string_slice,
    reason = "byte offsets come from .find()/.len() so they land on valid UTF-8 boundaries"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "offsets bounded by the .find() match position"
)]
fn declared_name(line: &str) -> Option<String> {
    let t = line.trim_start();
    for kw in [
        "pub const ",
        "pub static ",
        "pub fn ",
        "pub struct ",
        "const ",
        "static ",
        "let ",
        "fn ",
        "struct ",
        "enum ",
        "type ",
    ] {
        if let Some(rest) = t.strip_prefix(kw) {
            let ident: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !ident.is_empty() {
                return Some(ident);
            }
        }
    }
    if let Some(start) = line.find("env::var(") {
        let after = &line[start + "env::var(".len()..];
        if let Some(q) = after.find('"') {
            let key: String = after[q + 1..].chars().take_while(|c| *c != '"').collect();
            if !key.is_empty() {
                return Some(key);
            }
        }
    }
    None
}

#[expect(
    clippy::string_slice,
    reason = "eq comes from .find('=') so eq+1 is a valid char boundary"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "eq+1 bounded by the '=' match position"
)]
fn capture_value(line: &str) -> Option<String> {
    let eq = line.find('=')?;
    let rhs = line[eq + 1..].trim().trim_end_matches([';', ',']).trim();
    if rhs.is_empty() {
        return None;
    }
    Some(rhs.trim_matches('"').to_owned())
}

#[cfg(test)]
#[path = "walker_test.rs"]
mod walker_test;
