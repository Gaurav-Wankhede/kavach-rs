/// Check if a single dependency key is satisfied.
///
/// A blocker satisfies its dependent once its lifecycle is `Done`/`Verified`.
/// The status is parsed through the typed boundary accessor (`MemoryEntry::lifecycle`)
/// and the complete-set lives on the enum (`MemoryStatus::is_complete`) — a
/// non-canonical/absent status is `None` → NOT satisfied (fail-closed: a stale
/// row never silently unblocks a dependent).
#[must_use]
pub fn dep_key_satisfied(dep_key: &str, all: &[kavach_surreal::MemoryEntry]) -> bool {
    all.iter()
        .find(|e| e.entry_key == dep_key)
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
    use super::{is_dep_key_shaped, parse_declared_deps};

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
