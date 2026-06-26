//! Merge-conflict marker scan over the just-written content.
//!
//! ALGO line-prefix match; problem-class `conflict_marker_detect`. TIME `O(n)` SPACE `O(1)`.
//! SOURCE: <https://git-scm.com/docs/git-merge#_how_conflicts_are_presented>.

/// True when `content` carries a Git conflict marker (`<<<<<<<`, `=======`, `>>>>>>>`).
/// Matches only at line start to avoid flagging the markers in prose/docstrings.
pub(super) fn has_conflict_markers(content: &str) -> bool {
    content.lines().any(|line| {
        line.starts_with("<<<<<<< ") || line == "=======" || line.starts_with(">>>>>>> ")
    })
}
