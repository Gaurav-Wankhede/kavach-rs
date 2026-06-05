//! `to_lowercase()`-safe byte-offset mapping between an `lc` (lowercased) view of
//! a command and the original-case `command` string.
//!
//! `to_lowercase()` is NOT byte-length preserving (e.g. `İ`→`i̇`, 2→3 bytes), so an
//! offset computed in `lc` coordinates must be translated back before it can index
//! `command` — these helpers keep the two coordinate systems from desyncing.

/// Translate a byte offset in `lc` to the corresponding byte offset in `command`.
///
/// `to_lowercase()` maps each source char to one-or-more lowercase chars without
/// reordering, so walking both strings in lockstep — accumulating `lc` byte
/// length per source char — recovers the original-string offset for any `lc`
/// offset that lands on a char boundary.
pub(super) fn map_lc_offset_to_command(command: &str, lc: &str, target_lc: usize) -> Option<usize> {
    if target_lc == 0 {
        return Some(0);
    }
    let mut lc_acc = 0usize;
    for (cmd_off, ch) in command.char_indices() {
        if lc_acc >= target_lc {
            return Some(cmd_off);
        }
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "lc_acc accumulates per-char lowercase byte lengths of `command`; bounded by the lowercased string length, no overflow path"
        )]
        {
            lc_acc += ch.to_lowercase().map(char::len_utf8).sum::<usize>();
        }
    }
    // target at or past end maps to end-of-string.
    (lc_acc >= target_lc && target_lc <= lc.len()).then_some(command.len())
}
