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
/// key (incl. cross-project, no category prefix) is KEPT so the dependent
/// waits on the global pool. Prose that ran onto a `DEPENDS_ON:` line
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

/// Parse `DEPENDS_ON:` declarations from a card's content.
///
/// Convention: a line whose trimmed form starts with `DEPENDS_ON:`, followed by comma- or whitespace-separated keys, OR a
/// following indented `- key` bullet list. Tolerant: a card with no such
/// line yields an empty Vec.
#[must_use]
pub fn parse_declared_deps(content: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_dep_block = false;
    for raw in content.lines() {
        let line = raw.trim();
        let header = line.strip_prefix("DEPENDS_ON:");
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
#[path = "dep_key_test.rs"]
mod tests;
