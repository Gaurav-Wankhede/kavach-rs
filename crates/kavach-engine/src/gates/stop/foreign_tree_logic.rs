//! PURE dirty-set diff for the foreign-tree guard — split out so it is unit-testable
//! without spawning `git` (purity at the boundary; the impure `git status` read
//! lives in `phase/foreign_tree.rs`).

/// Count `git status --short` lines whose path is NOT attributable to THIS session
/// (`own_writes` = `session.files_modified`, compared by file BASENAME since the
/// session tracks basenames while git emits repo-relative paths). A high count means
/// another live session is mid-edit on the shared checkout.
///
/// Porcelain line shape: `XY <path>` (e.g. ` M src/foo.rs`, `?? a/b.rs`, with an
/// optional ` -> ` rename arrow). We take the final path token and match its
/// basename against `own_writes`.
#[must_use]
pub(crate) fn foreign_dirty_count(status: &str, own_writes: &[String]) -> usize {
    status
        .lines()
        .filter_map(line_path)
        .filter(|p| !is_own(p, own_writes))
        .count()
}

/// Extract the path from a porcelain `XY <path>` line (handles the ` -> ` rename
/// form by taking the destination). Empty/short lines yield `None`.
fn line_path(line: &str) -> Option<&str> {
    let rest = line.get(3..)?.trim();
    if rest.is_empty() {
        return None;
    }
    // Rename: "old -> new" — the live (dirty) file is the destination.
    Some(rest.rsplit(" -> ").next().unwrap_or(rest).trim())
}

/// True iff `path`'s basename matches one of this session's own writes (which are
/// stored as basenames). A path with no own-write basename is foreign.
fn is_own(path: &str, own_writes: &[String]) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    own_writes.iter().any(|w| {
        let wb = w.rsplit('/').next().unwrap_or(w);
        wb == base
    })
}

#[cfg(test)]
mod tests {
    use super::foreign_dirty_count;

    #[test]
    fn clean_tree_has_zero_foreign() {
        assert_eq!(foreign_dirty_count("", &[]), 0);
    }

    #[test]
    fn own_writes_only_are_not_foreign() {
        // Both dirty files are this session's own (by basename) → zero foreign.
        let status = " M crates/a/src/lib.rs\n M crates/b/src/main.rs\n";
        let own = vec!["lib.rs".to_owned(), "main.rs".to_owned()];
        assert_eq!(foreign_dirty_count(status, &own), 0);
    }

    #[test]
    fn another_sessions_files_count_as_foreign() {
        // Three dirty files, none written by this session → all three foreign.
        let status = " M x/foo.rs\n?? y/bar.astro\n M z/baz.toml\n";
        assert_eq!(foreign_dirty_count(status, &[]), 3);
    }

    #[test]
    fn mixed_counts_only_the_foreign_ones() {
        let status = " M src/mine.rs\n M src/theirs.rs\n?? other/new.rs\n";
        let own = vec!["mine.rs".to_owned()];
        assert_eq!(foreign_dirty_count(status, &own), 2, "theirs.rs + new.rs are foreign");
    }

    #[test]
    fn rename_line_uses_the_destination_path() {
        // Rename "old -> new": the live file is the destination basename.
        let status = "R  src/old.rs -> src/new.rs\n";
        let own_dest = vec!["new.rs".to_owned()];
        assert_eq!(foreign_dirty_count(status, &own_dest), 0, "dest is mine → not foreign");
        assert_eq!(foreign_dirty_count(status, &[]), 1, "no own match → foreign");
    }
}
