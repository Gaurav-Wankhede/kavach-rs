use std::path::Path;

use super::error::RagError;
use super::node::TreeNode;
use super::scanner::{ScannedDoc, scan_dir};
use super::tree::RagTree;

/// Scan `root` for markdown files and build one [`RagTree`] per file.
///
/// The resulting trees carry empty `summary` fields; feed them through
/// [`crate::protocol::pending_requests`] to obtain `SummaryRequest`s for
/// an external summarizer, then apply responses via
/// [`crate::protocol::apply_summaries`] before serializing.
///
/// # Errors
/// Returns `RagError::Invalid` if the directory scan fails or if no markdown files are found.
pub fn build_trees_from_dir(root: &Path, source_label: &str) -> Result<Vec<RagTree>, RagError> {
    let docs = scan_dir(root, &["md"]).map_err(|e| RagError::Invalid(e.to_string()))?;
    let mut out: Vec<RagTree> = Vec::with_capacity(docs.len());
    for doc in docs {
        let root_node = from_scanned(&doc)?;
        out.push(RagTree::new(source_label, root_node));
    }
    Ok(out)
}

fn from_scanned(doc: &ScannedDoc) -> Result<TreeNode, RagError> {
    from_markdown(doc.id(), doc.body())
}

/// Build a skeleton tree from a markdown document by splitting on headings.
///
/// Each `#`-prefixed heading becomes a tree node; nesting follows heading
/// depth (`#` < `##` < `###`). The `body` field holds the raw text between
/// the heading and the next heading of equal or shallower depth.
///
/// The resulting tree has empty `summary` and `keywords` — those fields are
/// filled in by Phase B's offline LLM summarizer via the `protocol` module.
///
/// # Errors
/// Returns `RagError::Invalid` if `source_id` is empty.
pub fn from_markdown(source_id: &str, body: &str) -> Result<TreeNode, RagError> {
    if source_id.is_empty() {
        return Err(RagError::Invalid("source_id must not be empty".into()));
    }
    let sections = parse_sections(body);
    let root_children = build_forest(&sections, 0, sections.len(), 1, source_id);
    Ok(TreeNode {
        id: source_id.to_owned(),
        title: source_id.to_owned(),
        summary: String::new(),
        keywords: Vec::new(),
        file_patterns: Vec::new(),
        body: String::new(),
        children: root_children,
    })
}

#[derive(Debug, Clone)]
struct Section {
    depth: usize,
    title: String,
    body: String,
}

fn parse_sections(source: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;
    for line in source.lines() {
        if let Some((depth, title)) = heading_parts(line) {
            if let Some(done) = current.take() {
                sections.push(done);
            }
            current = Some(Section {
                depth,
                title,
                body: String::new(),
            });
            continue;
        }
        if let Some(ref mut sec) = current {
            sec.body.push_str(line);
            sec.body.push('\n');
        }
    }
    if let Some(done) = current {
        sections.push(done);
    }
    sections
}

fn heading_parts(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    let depth = trimmed.chars().take_while(|c| *c == '#').count();
    if depth == 0 || depth > 6 {
        return None;
    }
    let rest = trimmed.get(depth..)?;
    if !rest.starts_with(' ') {
        return None;
    }
    Some((depth, rest.trim().to_owned()))
}

fn build_forest(
    sections: &[Section],
    start: usize,
    end: usize,
    depth: usize,
    parent_id: &str,
) -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let Some(section) = sections.get(cursor) else {
            break;
        };
        if section.depth != depth {
            cursor = cursor.saturating_add(1);
            continue;
        }
        let mut child_end = cursor.saturating_add(1);
        while child_end < end {
            match sections.get(child_end) {
                Some(next) if next.depth > depth => child_end = child_end.saturating_add(1),
                Some(_) | None => break,
            }
        }
        let next_depth = depth.saturating_add(1);
        let id = format!("{parent_id}#{}", slugify(&section.title));
        let children = build_forest(
            sections,
            cursor.saturating_add(1),
            child_end,
            next_depth,
            &id,
        );
        nodes.push(TreeNode {
            id,
            title: section.title.clone(),
            summary: String::new(),
            keywords: Vec::new(),
            file_patterns: Vec::new(),
            body: section.body.trim_end().to_owned(),
            children,
        });
        cursor = child_end;
    }
    nodes
}

fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
