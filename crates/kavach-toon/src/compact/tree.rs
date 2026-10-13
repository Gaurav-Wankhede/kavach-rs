//! Token-optimal hierarchical path tree formatter for TOON.
//!
//! Enforces:
//! - Strict 2-space indentation (pre-merged in modern BPE tokenizers).
//! - Single-child prefix collapsing (e.g. `crates/kavach-cli/src/` instead of 3 nested lines).
//! - Zero trailing spaces and zero blank lines.

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Default)]
struct PathNode {
    children: BTreeMap<String, PathNode>,
    status: Option<String>,
}

impl PathNode {
    fn insert(&mut self, path: &str, status: Option<String>) {
        let clean = path.strip_prefix("./").unwrap_or(path);
        let segments: Vec<&str> = clean.split("/").filter(|s| !s.is_empty()).collect();
        let mut curr = self;
        for (idx, seg) in segments.iter().enumerate() {
            let is_last = idx + 1 == segments.len();
            curr = curr.children.entry((*seg).to_string()).or_default();
            if is_last {
                curr.status = status.clone().or_else(|| curr.status.clone());
            }
        }
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for (name, child) in &self.children {
            Self::render_node(name, child, 0, &mut out);
        }
        out
    }

    fn render_node(name: &str, node: &PathNode, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        if node.children.is_empty() {
            if let Some(status) = &node.status {
                let _ = writeln!(out, "{indent}{status} {name}");
            } else {
                let _ = writeln!(out, "{indent}{name}");
            }
        } else {
            let mut prefix = name.to_string();
            let mut curr = node;
            while curr.children.len() == 1 && curr.status.is_none() {
                let (next_name, next_node) = match curr.children.iter().next() {
                    Some(pair) => pair,
                    None => break,
                };
                if next_node.children.is_empty() {
                    break;
                }
                prefix = format!("{prefix}/{next_name}");
                curr = next_node;
            }

            if let Some(status) = &curr.status {
                let _ = writeln!(out, "{indent}{status} {prefix}/");
            } else {
                let _ = writeln!(out, "{indent}{prefix}/");
            }

            for (c_name, c_node) in &curr.children {
                Self::render_node(c_name, c_node, depth + 1, out);
            }
        }
    }
}

/// Format an iterator or slice of (path, optional_status) into a token-optimal tree.
#[must_use]
pub fn encode_path_tree<'a, I>(entries: I) -> String
where
    I: IntoIterator<Item = (&'a str, Option<&'a str>)>,
{
    let mut tree = PathNode::default();
    for (path, status) in entries {
        tree.insert(path, status.map(ToString::to_string));
    }
    tree.render()
}
