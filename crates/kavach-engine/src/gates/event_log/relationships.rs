//! Typed inter-entry relationship extraction from row content. Two surfaces:
//! YAML frontmatter (`frontmatter` submodule) and Markdown typed wikilinks
//! (`[[memory:slug/cat/key]]` → a `references` edge).
mod frontmatter;
mod nlu;
#[cfg(test)]
#[path = "relationships_test.rs"]
mod tests;
use frontmatter::extract_frontmatter_rels;
use nlu::extract_nlu_rels;
/// A typed relationship extracted from row content.
///
/// `rel` is one of: `depends_on`, blocks, supersedes, references. `target` is
/// either a fully-qualified entity name `<project_slug>/<category>/<key>` (from
/// a `[[memory:slug/cat/key]]` wikilink) OR a bare key (from a frontmatter
/// directive — caller resolves to the current project + same-category).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExtractedRelationship {
    pub rel: String,
    pub target: String,
    /// `true` only for NLU-prose-harvested edges — resolve-or-drop before admitting.
    pub speculative: bool,
}
impl ExtractedRelationship {
    /// Construct a non-speculative edge — the canonical cross-crate entry point.
    #[must_use]
    pub fn new(rel: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            rel: rel.into(),
            target: target.into(),
            speculative: false,
        }
    }
}
/// Extract typed inter-entry relationships from row content.
///
/// Sources: frontmatter keys, `[[memory:...]]` wikilinks, and NLU prose cues
/// ("depends on X"). Sorted + deduped so duplicate arcs from multiple surfaces
/// collapse to one edge.
#[must_use]
pub fn extract_memory_entry_relationships(content: &str) -> Vec<ExtractedRelationship> {
    let mut out: Vec<ExtractedRelationship> = Vec::new();
    extract_frontmatter_rels(content, &mut out);
    extract_typed_wikilinks(content, &mut out);
    extract_nlu_rels(content, &mut out);
    out.sort_unstable_by(|a, b| a.rel.cmp(&b.rel).then_with(|| a.target.cmp(&b.target)));
    out.dedup();
    out
}
fn extract_typed_wikilinks(content: &str, out: &mut Vec<ExtractedRelationship>) {
    for line in content.lines() {
        let mut rest = line;
        while let Some(start) = rest.find("[[memory:") {
            let Some(after_open) = rest.get(start.saturating_add(2)..) else {
                break;
            };
            let Some(end) = after_open.find("]]") else {
                break;
            };
            let Some(name) = after_open.get(..end) else {
                break;
            };
            let Some(qname) = name.trim().strip_prefix("memory:") else {
                break;
            };
            let q = qname.trim();
            if !q.is_empty() {
                out.push(ExtractedRelationship::new("references", q));
            }
            let Some(s) = after_open.get(end.saturating_add(2)..) else {
                break;
            };
            rest = s;
        }
    }
}
