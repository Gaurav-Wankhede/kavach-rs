//! Shell-segment command-position helpers shared by the write-bypass detectors.
//! A "segment" is the text between shell separators (`&`, `|`, `;`, `(`, `{`,
//! newline); command-position detection keys off a segment's first word so
//! `sed`/`&& sed`/`| sed`/`(sed` match but `sedan`/`grep sediment` do not.

/// Return true when any segment of `lower` has `name` as its first word.
pub(super) fn segment_first_word_is(lower: &str, name: &str) -> bool {
    let separators = ['&', '|', ';', '(', '{', '\n'];
    for segment in lower.split(|c: char| separators.contains(&c)) {
        if segment.split_whitespace().next() == Some(name) {
            return true;
        }
    }
    false
}

/// True when `tool` is in command position and `flag` appears as a standalone
/// token among its arguments (handles `curl -O url`).
pub(super) fn segment_has_flag(lower: &str, tool: &str, flag: &str) -> bool {
    let separators = ['&', '|', ';', '(', '{', '\n'];
    for seg in lower.split(|c: char| separators.contains(&c)) {
        let mut words = seg.split_whitespace();
        if words.next() == Some(tool) && words.any(|w| w == flag) {
            return true;
        }
    }
    false
}
