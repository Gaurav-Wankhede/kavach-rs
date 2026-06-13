//! High-precision destructive-intent detector — the feature behind the
//! `destructive/critical` classification leaf.
//!
//! Destructive intent needs a destructive shell IDIOM, or a destructive VERB
//! (word-boundary — "dropdown" never trips "drop") within 4 tokens of a
//! destructive TARGET artifact. A bare verb in prose ("remove the noise from
//! the gates") is an edit request, not a destructive op — labeling it
//! destructive/critical was the gate-noise false positive; the verb+target
//! pair is what the `PreToolUse` Bash guard actually blocks.

const VERBS: [&str; 7] = [
    "delete", "remove", "drop", "destroy", "purge", "truncate", "wipe",
];

const TARGETS: [&str; 24] = [
    "file",
    "files",
    "folder",
    "directory",
    "table",
    "tables",
    "database",
    "db",
    "data",
    "branch",
    "repo",
    "repository",
    "row",
    "rows",
    "record",
    "records",
    "history",
    "backup",
    "backups",
    "bucket",
    "volume",
    "prod",
    "production",
    "account",
];

const IDIOMS: [&str; 6] = [
    "rm -rf",
    "rm -r ",
    "force push",
    "git reset --hard",
    "drop table",
    "delete from",
];

/// True iff the lowercased prompt carries a destructive idiom or a
/// verb-near-target pair (window: 4 tokens after the verb).
pub(super) fn has_destructive(s: &str) -> bool {
    if IDIOMS.iter().any(|i| s.contains(i)) {
        return true;
    }
    let tokens: Vec<&str> = s.split(|c: char| !c.is_alphanumeric()).collect();
    tokens.iter().enumerate().any(|(i, tok)| {
        VERBS.contains(tok)
            && tokens
                .iter()
                .skip(i.saturating_add(1))
                .take(4)
                .any(|t| TARGETS.contains(t))
    })
}
