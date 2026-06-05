//! First-word-of-segment matching: split a command on shell separators and test
//! whether any segment's leading word is one of a set of command names.

/// Return true when any command-position segment in `lc` has first word equal to `name`.
///
/// Convenience wrapper around `first_word_matches` for the single-name case.
pub(crate) fn first_word_is(lc: &str, name: &str) -> bool {
    first_word_matches(lc, &[name])
}

/// Generalized segment-split helper — splits on shell separators and checks
/// whether any segment's first word matches any of the given names.
///
/// Separators: `&`, `|`, `;`, `(`, `{`, `\n`.
pub(crate) fn first_word_matches(lc: &str, names: &[&str]) -> bool {
    let separators = ['&', '|', ';', '(', '{', '\n'];
    for segment in lc.split(|c: char| separators.contains(&c)) {
        let Some(first_word) = segment.split_whitespace().next() else {
            continue;
        };
        if names.contains(&first_word) {
            return true;
        }
    }
    false
}
