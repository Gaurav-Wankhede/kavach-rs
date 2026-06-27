//! Typed inter-entry relationship extraction from row content. Two surfaces:
//! YAML frontmatter (`frontmatter` submodule) and Markdown typed wikilinks
//! (`[[memory:slug/cat/key]]` → a `references` edge).
mod frontmatter;
mod nlu;
#[cfg(test)]
#[path = "relationships_test.rs"]
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
}
impl ExtractedRelationship {
    /// Construct an edge. The canonical cross-crate entry point — the struct is
    /// `#[non_exhaustive]`, so downstream crates (e.g. the CLI `--depends-on`
    /// merge) cannot use a struct literal and must route through here.
    #[must_use]
    pub fn new(rel: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            rel: rel.into(),
            target: target.into(),
        }
    }
}
/// Extract typed inter-entry relationships from row content.
///
/// Sources: frontmatter keys, `[[memory:...]]` wikilinks, and NLU prose cues
/// ("depends on X"). Sorted + deduped so duplicate arcs from multiple surfaces
/// collapse to one edge.
///
/// `ALGO`: append-from-3-extractors → `sort_unstable` + dedup.
/// `PROBLEM_CLASS`: small-set dedup (n = edges per row, typically < 20).
/// `REJECTED`: `HashSet` insert (alloc + hash cost dominates at n<20; sort+dedup
/// wins cache locality); `BTreeSet` (`O(n log n)` with worse constant than
/// `sort_unstable` on a `Vec`).
/// TIME: `O(n log n)` | SPACE: `O(n)` | YEAR: 2026 | SEARCHED: 2026-06
/// BENCHMARK: <https://github.com/rust-lang/rust/blob/master/library/alloc/src/slice.rs> (`sort_unstable` pdqsort)
/// SOURCE: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.dedup>
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
                out.push(ExtractedRelationship {
                    rel: String::from("references"),
                    target: q.to_owned(),
                });
            }
            let Some(s) = after_open.get(end.saturating_add(2)..) else {
                break;
            };
            rest = s;
        }
    }
}
#[cfg(test)]
mod supersedes_extraction_tests {
    use super::extract_memory_entry_relationships as extract;
    fn has_supersedes(body: &str, target: &str) -> bool {
        extract(body)
            .iter()
            .any(|e| e.rel == "supersedes" && e.target == target)
    }
    #[test]
    fn fenced_frontmatter_yields_supersedes() {
        let body = "---\nsupersedes: dioxus-0.7-websys-gap\n---\nbody\n";
        assert!(
            has_supersedes(body, "dioxus-0.7-websys-gap"),
            "{:?}",
            extract(body)
        );
    }
    #[test]
    fn loose_leading_kv_yields_supersedes() {
        let body = "supersedes: dioxus-0.7-websys-gap\n";
        assert!(
            has_supersedes(body, "dioxus-0.7-websys-gap"),
            "{:?}",
            extract(body)
        );
    }
    #[test]
    fn nlu_prose_yields_supersedes() {
        let body = "This supersedes dioxus-0.7-websys-gap; adopt use_route.\n";
        assert!(
            has_supersedes(body, "dioxus-0.7-websys-gap"),
            "{:?}",
            extract(body)
        );
    }
}
