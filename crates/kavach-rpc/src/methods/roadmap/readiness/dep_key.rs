/// Normalize a dependency key to its bare entry-key tail for comparison.
///
/// A `--depends-on` value reaches the card content in EITHER a bare form
/// (`roadmap.phasemerge.w1-...`, the stored `entry_key`) OR a project-qualified
/// form (`kavach-rs/roadmap/roadmap.phasemerge.w1-...`) the kanban renders. Exact
/// `entry_key == dep_key` then never matches the qualified form, so the dependent
/// wedges in WAITING forever (DAG ghost-edge). Stripping everything up to and
/// including the last `/` collapses both forms to the bare `entry_key`, so the
/// resolver compares like with like. A bare key (no `/`) is returned unchanged.
#[must_use]
pub fn bare_dep_key(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}

/// Check if a single dependency key is satisfied.
///
/// A blocker satisfies its dependent once its lifecycle is `Done`/`Verified`.
/// The status is parsed through the typed boundary accessor (`MemoryEntry::lifecycle`)
/// and the complete-set lives on the enum (`MemoryStatus::is_complete`) — a
/// non-canonical/absent status is `None` → NOT satisfied (fail-closed: a stale
/// row never silently unblocks a dependent).
///
/// Both the dep key and each candidate `entry_key` are normalized to their bare
/// tail (`bare_dep_key`) so a project-qualified dep (`<project>/<category>/<key>`)
/// resolves against the bare stored `entry_key` instead of wedging the DAG.
#[must_use]
pub fn dep_key_satisfied(dep_key: &str, all: &[kavach_surreal::MemoryEntry]) -> bool {
    let want = bare_dep_key(dep_key);
    all.iter()
        .find(|e| bare_dep_key(&e.entry_key) == want)
        .and_then(kavach_surreal::MemoryEntry::lifecycle)
        .is_some_and(kavach_types::MemoryStatus::is_complete)
}

/// `true` when `tok` is key-shaped, not prose.
///
/// A card key is key-safe chars only (alnum / `-` `_` `.` `/`); an absent-but-real
/// key (incl. cross-project, no category prefix) is KEPT so the dependent shows
/// BLOCKED against the global pool. Prose that ran onto a `DEPENDS_ON:` line
/// (`SCOPE:`, `(a)`, `&Surreal<Db>`) carries markup/`:` and is rejected, so it
/// can't wedge the dispatch DAG with a phantom node. Trailing `.` = a sentence,
/// not a key.
#[must_use]
pub fn is_dep_key_shaped(tok: &str) -> bool {
    !tok.is_empty()
        && !tok.ends_with('.')
        && tok
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

/// Parse `BLOCKED_BY:` / `DEPENDS_ON:` declarations from a card's content.
///
/// Convention: a line whose trimmed form starts with `BLOCKED_BY:` or
/// `DEPENDS_ON:`, followed by comma- or whitespace-separated keys, OR a
/// following indented `- key` bullet list. Tolerant: a card with no such
/// line yields an empty Vec.
#[must_use]
pub fn parse_declared_deps(content: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_dep_block = false;
    for raw in content.lines() {
        let line = raw.trim();
        let header = line
            .strip_prefix("BLOCKED_BY:")
            .or_else(|| line.strip_prefix("DEPENDS_ON:"));
        if let Some(rest) = header {
            in_dep_block = true;
            // Admit ONLY card-key-shaped tokens: a `DEPENDS_ON:` line may run into
            // prose (`DEPENDS_ON: foo.bar SCOPE: ...`), and an unvalidated prose
            // token becomes a permanently-missing DAG node that wedges dispatch.
            for tok in rest.split([',', ' ', '\t']) {
                let key = tok.trim();
                if is_dep_key_shaped(key) {
                    deps.push(key.to_owned());
                }
            }
            continue;
        }
        if in_dep_block {
            if let Some(bullet) = line.strip_prefix("- ") {
                if let Some(key) = bullet.split_whitespace().next()
                    && is_dep_key_shaped(key)
                {
                    deps.push(key.to_owned());
                }
                continue;
            }
            if !line.is_empty() {
                in_dep_block = false;
            }
        }
    }
    deps
}

#[cfg(test)]
mod tests {
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
        assert!(dep_key_satisfied("roadmap.phasemerge.w1-db-phase-config", &all));
    }

    #[test]
    fn unsatisfied_prereq_stays_blocked_via_both_forms() {
        // Fail-closed: an incomplete prereq blocks the dependent in EITHER key form.
        let all = [entry("roadmap.phasemerge.w1-db-phase-config", "in_progress")];
        assert!(!dep_key_satisfied(
            "kavach-rs/roadmap/roadmap.phasemerge.w1-db-phase-config",
            &all
        ));
        assert!(!dep_key_satisfied("roadmap.phasemerge.w1-db-phase-config", &all));
    }

    #[test]
    fn real_and_bare_keys_are_shaped() {
        // Category-prefixed keys AND bare/cross-project keys (no prefix) are kept:
        // an absent-but-real key must stay as a phantom prereq (global-pool resolve).
        assert!(is_dep_key_shaped("decision.surreal-read-path-retry-increment-2"));
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
        let content = "BLOCKED_BY:\n- roadmap.real-one\n- <not-a-key>\n- decision.real-two";
        let mut deps = parse_declared_deps(content);
        deps.sort();
        assert_eq!(deps, vec!["decision.real-two", "roadmap.real-one"]);
    }
}
