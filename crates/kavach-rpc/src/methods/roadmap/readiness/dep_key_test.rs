use super::{bare_dep_key, dep_key_satisfied, is_dep_key_shaped, parse_declared_deps};

fn entry(key: &str, status: &str) -> kavach_surreal::MemoryEntry {
    // Mirror kavach_surreal::dual_write tests' `with_status` literal: only the
    // two fields the resolver reads (entry_key, entry_status) carry meaning;
    // the rest take their empty/None form.
    kavach_surreal::MemoryEntry {
        id: None,
        project: surrealdb_types::RecordId::new("project", "t"),
        category: Some("roadmap".into()),
        entry_key: key.to_owned(),
        title: "t".to_owned(),
        content: String::new(),
        status: None,
        entry_status: Some(status.to_owned()),
        tags: None,
        decay_score: None,
        access_count: None,
        created_at: None,
        updated_at: None,
        priority: None,
        lane: None,
        exec_prompt: None,
        occupied_by: None,
        occupied_until: None,
    }
}

#[test]
fn bare_key_strips_project_category_prefix() {
    assert_eq!(
        bare_dep_key("kavach-rs/roadmap/roadmap.phasemerge.w1-db-phase-config"),
        "roadmap.phasemerge.w1-db-phase-config"
    );
    // A bare key is returned unchanged (no `/`).
    assert_eq!(
        bare_dep_key("roadmap.phasemerge.w1-db-phase-config"),
        "roadmap.phasemerge.w1-db-phase-config"
    );
}

#[test]
fn qualified_dep_resolves_against_bare_entry_key() {
    // Regression for the DAG ghost-edge that wedged W2: a project-qualified
    // dep string must resolve against the bare stored entry_key once the
    // prerequisite is verified.
    let all = [entry("roadmap.phasemerge.w1-db-phase-config", "verified")];
    assert!(
        dep_key_satisfied(
            "kavach-rs/roadmap/roadmap.phasemerge.w1-db-phase-config",
            &all
        ),
        "qualified dep must match bare entry_key when prereq is verified"
    );
    // Bare form still resolves identically.
    assert!(dep_key_satisfied(
        "roadmap.phasemerge.w1-db-phase-config",
        &all
    ));
}

#[test]
fn unsatisfied_prereq_stays_blocked_via_both_forms() {
    // Fail-closed: an incomplete prereq blocks the dependent in EITHER key form.
    let all = [entry(
        "roadmap.phasemerge.w1-db-phase-config",
        "in_progress",
    )];
    assert!(!dep_key_satisfied(
        "kavach-rs/roadmap/roadmap.phasemerge.w1-db-phase-config",
        &all
    ));
    assert!(!dep_key_satisfied(
        "roadmap.phasemerge.w1-db-phase-config",
        &all
    ));
}

#[test]
fn real_and_bare_keys_are_shaped() {
    // Category-prefixed keys AND bare/cross-project keys (no prefix) are kept:
    // an absent-but-real key must stay as a phantom prereq (global-pool resolve).
    assert!(is_dep_key_shaped(
        "decision.surreal-read-path-retry-increment-2"
    ));
    assert!(is_dep_key_shaped("roadmap.unit.iwi-feedback-inbound-e5"));
    assert!(is_dep_key_shaped("u1"));
    assert!(is_dep_key_shaped("ghost"));
}

#[test]
fn prose_tokens_are_rejected() {
    // The exact fragments that wedged the dispatch DAG into ALL_BLOCKED: prose
    // carries markup/`:`/whitespace, or trails a sentence dot.
    for tok in [
        "&Surreal<Db>",
        "Arc<ArcSwap<Surreal<Db>>>)",
        "(a)",
        "SCOPE:",
        "decision.surreal-read-path-retry-increment-2.", // trailing dot = sentence
        "",
    ] {
        assert!(!is_dep_key_shaped(tok), "{tok} must NOT be a dep key");
    }
}

#[test]
fn depends_on_line_drops_markup_prose_that_wedged_the_dag() {
    // Regression for the loop-wedge: the markup-bearing fragments that became
    // phantom DAG nodes (`Surreal<Db>`, `(a)`, `ArcSwap<...>`, `SCOPE:`) must
    // NOT survive. Bare words can't be told apart from bare keys (u1/ghost), so
    // the guard targets markup; clean keys-only authoring is the pattern.
    let content = "DEPENDS_ON: &Surreal<Db>, Arc<ArcSwap<Surreal<Db>>>) SCOPE: (a)";
    let deps = parse_declared_deps(content);
    assert!(
        deps.is_empty(),
        "every markup token must be rejected, not poison the DAG: {deps:?}"
    );
}

#[test]
fn clean_comma_list_parses_all_keys() {
    let content = "DEPENDS_ON: roadmap.a, decision.b roadmap.c";
    let mut deps = parse_declared_deps(content);
    deps.sort();
    assert_eq!(deps, vec!["decision.b", "roadmap.a", "roadmap.c"]);
}

#[test]
fn bullet_list_takes_first_token_of_each_bullet() {
    // Bullet path takes the first whitespace token of each `- ` line. A bullet
    // whose first token carries markup drops; a bare-word first token is kept
    // (it can't be distinguished from a bare key u1/ghost — same shape).
    let content = "DEPENDS_ON:\n- roadmap.real-one\n- <not-a-key>\n- decision.real-two";
    let mut deps = parse_declared_deps(content);
    deps.sort();
    assert_eq!(deps, vec!["decision.real-two", "roadmap.real-one"]);
}
