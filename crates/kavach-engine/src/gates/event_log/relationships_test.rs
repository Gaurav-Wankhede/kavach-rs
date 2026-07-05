//! Relationship-extraction regression tests: frontmatter (fenced + loose),
//! typed memory wikilinks, key filtering, and sort/dedup.

use super::{ExtractedRelationship, extract_memory_entry_relationships};

#[test]
fn should_parse_yaml_frontmatter_with_fences() {
    let c = "---\ndepends_on: [foo, bar]\nblocks: baz\n---\nbody";
    let rels = extract_memory_entry_relationships(c);
    assert_eq!(rels.len(), 3);
    assert!(rels.contains(&ExtractedRelationship::new("depends_on", "foo")));
    assert!(rels.contains(&ExtractedRelationship::new("blocks", "baz")));
}

#[test]
fn should_parse_loose_frontmatter_without_fences() {
    let c = "depends_on: alpha\nblocks: beta\n\nfree-form body here";
    let rels = extract_memory_entry_relationships(c);
    assert_eq!(rels.len(), 2);
}

#[test]
fn should_extract_memory_wikilinks() {
    let c = "see [[memory:proj/decision/auth-pivot]] and [[memory:proj/roadmap/foo]]";
    let rels = extract_memory_entry_relationships(c);
    assert_eq!(rels.len(), 2);
    assert!(rels.iter().all(|r| r.rel == "references"));
}

#[test]
fn should_ignore_non_relationship_keys() {
    let c = "title: ignored\nauthor: also ignored\n";
    assert!(extract_memory_entry_relationships(c).is_empty());
}

#[test]
fn should_dedupe_and_sort() {
    let c = "depends_on: [b, a, b]";
    let rels = extract_memory_entry_relationships(c);
    assert_eq!(rels.len(), 2);
    assert_eq!(rels[0].target, "a");
    assert_eq!(rels[1].target, "b");
}
