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
fn short_bare_token_rejected_key_shaped_token_accepted() {
    assert!(rels("after ab").is_empty(), "2-char token too short");
    // Key-shaped bare tokens are accepted: a digit-bearing id (`g1a`, the w2
    // wave-label convention) and internal-case identifiers.
    assert!(has(&rels("depends on g1a"), "depends_on", "g1a"));
    assert!(has(
        &rels("depends on dagGap1"),
        "depends_on",
        "dagGap1"
    ));
}

/// REGRESSION: live kavach-rs data carried speculative edges to ordinary English
/// words that merely followed a cue verb. Each became a permanent `(unresolved)`
/// DAG node. The stop list could not keep up; the gate is now structural.
///
/// NOTE: `read-back` is deliberately NOT in the rejected set — it is
/// kebab-shaped and therefore indistinguishable from a real key at this layer.
/// That case is caught downstream by `resolve_speculative_deps` (resolve-or-drop
/// against known keys), which is why FIX A matters as much as this one.
#[test]
fn prose_words_after_a_verb_are_rejected() {
    // Real targets harvested from live kavach-rs rows before the fix.
    for word in ["their", "dedup", "every", "model", "rename", "binary", "hooks"] {
        let out = rels(&format!("Parent cleaned exit.rs; blocks {word} now."));
        assert!(
            !has(&out, "blocks", word),
            "prose word {word:?} must not become an edge, got {out:?}"
        );
    }
}

/// The structural rule must not over-reject: qnames, kebab, dotted, snake and
/// digit-bearing keys all remain valid targets.
#[test]
fn key_shaped_targets_still_harvested() {
    assert!(has(
        &rels("depends on kavach-rs/roadmap/tf-entity-extraction"),
        "depends_on",
        "kavach-rs/roadmap/tf-entity-extraction"
    ));
    assert!(has(&rels("after stage1-backend"), "depends_on", "stage1-backend"));
    assert!(has(&rels("requires v0.3.cli"), "depends_on", "v0.3.cli"));
    assert!(has(&rels("requires v0_3_cli"), "depends_on", "v0_3_cli"));
    // ≥3 chars (the length floor) AND digit-bearing.
    assert!(has(&rels("after g1a"), "depends_on", "g1a"));
    assert!(has(&rels("after w12"), "depends_on", "w12"));
}

#[test]
fn nlu_edges_are_marked_speculative() {
    let out = rels("This depends on dag-gap1-critical-path.");
    assert!(out.iter().all(|r| r.speculative), "{out:?}");
}
