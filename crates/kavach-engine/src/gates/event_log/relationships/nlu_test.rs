use super::super::ExtractedRelationship;
use super::extract_nlu_rels;

fn rels(content: &str) -> Vec<ExtractedRelationship> {
    let mut out = Vec::new();
    extract_nlu_rels(content, &mut out);
    out
}

fn has(out: &[ExtractedRelationship], rel: &str, target: &str) -> bool {
    out.iter().any(|r| r.rel == rel && r.target == target)
}

#[test]
fn depends_on_phrase_yields_edge() {
    let out = rels("This card depends on dag-gap1-critical-path.");
    assert!(has(&out, "depends_on", "dag-gap1-critical-path"));
}

#[test]
fn requires_phrase_is_depends_on() {
    let out = rels("It requires proj/roadmap/edge-authoring before starting.");
    assert!(has(&out, "depends_on", "proj/roadmap/edge-authoring"));
}

#[test]
fn blocked_by_phrase_yields_blocks_edge() {
    let out = rels("Work here is blocked by gate-binary-rebuild.");
    assert!(has(&out, "blocks", "gate-binary-rebuild"));
}

#[test]
fn after_sequencing_phrase_yields_depends_on() {
    let out = rels("Run this after split-oversized-files completes.");
    assert!(has(&out, "depends_on", "split-oversized-files"));
}

#[test]
fn supersedes_and_replaces_map_to_supersedes() {
    let out = rels("This supersedes old-design-v1 and replaces legacy-approach.");
    assert!(has(&out, "supersedes", "old-design-v1"));
    assert!(has(&out, "supersedes", "legacy-approach"));
}

#[test]
fn trailing_punctuation_is_stripped() {
    let out = rels("Depends on dag-gap1, then ship.");
    assert!(has(&out, "depends_on", "dag-gap1"));
    assert!(!has(&out, "depends_on", "dag-gap1,"));
}

#[test]
fn stopwords_after_verb_are_rejected() {
    // "after that", "requires it", "depends on the" must NOT create edges —
    // false arcs poison the DAG topo-sort.
    let out = rels("Do it after that. It requires it. Depends on the review.");
    assert!(
        out.is_empty(),
        "stopword targets must be rejected, got {out:?}"
    );
}

#[test]
fn short_bare_token_rejected_long_slug_accepted() {
    assert!(rels("after ab").is_empty(), "2-char token too short");
    // a long single bare token with no separator is still accepted (could be a key)
    assert!(has(
        &rels("depends on edgeauthoring"),
        "depends_on",
        "edgeauthoring"
    ));
}

#[test]
fn nlu_edges_are_marked_speculative() {
    let out = rels("This depends on dag-gap1-critical-path.");
    assert!(out.iter().all(|r| r.speculative), "{out:?}");
}
