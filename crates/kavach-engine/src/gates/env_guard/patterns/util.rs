//! Shared predicate across env- and dotenv-pattern checks.

/// True when a search strips values to names only (`grep -o '^[^=]*'` / awk key).
pub(super) fn is_names_only(lc: &str) -> bool {
    (lc.contains("grep -o") && lc.contains("^[^=]*"))
        || (lc.contains("awk") && lc.contains("print $1"))
}
