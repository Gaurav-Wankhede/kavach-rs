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
    statuses: Vec<String>,
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
                if let Some(s) = status.clone() { curr.statuses.push(s); }
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
            if node.statuses.is_empty() {
                let _ = writeln!(out, "{indent}{name}");
            } else if node.statuses.len() == 1 {
                let _ = writeln!(out, "{indent}{} {name}", node.statuses[0]);
            } else {
                let _ = writeln!(out, "{indent}{name}");
                let sub_indent = "  ".repeat(depth + 1);
                for st in &node.statuses {
                    let _ = writeln!(out, "{sub_indent}{st}");
                }
            }
        } else {
            let mut prefix = name.to_string();
            let mut curr = node;
            while curr.children.len() == 1 && curr.statuses.is_empty() {
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

            let _ = writeln!(out, "{indent}{prefix}/");

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
