//! Locate where the downstream command begins after a command-position
//! `source `/`. ` invocation, in `lc` (lowercased) coordinates.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "bounded byte-offset scanner: arithmetic is occurrence-position index math over the command string, guarded by .find()/.len(); no overflow path"
)]
use crate::gates::env_guard_shell_parse::{is_command_position, skip_shell_redirects};

/// Locate the byte offset *within `lc`* where the downstream command begins,
/// for the FIRST command-position `needle` hit followed by a `&&`/`;` separator.
///
/// All scanning/slicing here stays in `lc` coordinates: `to_lowercase()` is not
/// byte-length preserving (e.g. `İ`→`i̇`, 2→3 bytes), so an `lc` offset must
/// never index `command` directly — the caller maps it back instead.
pub(super) fn downstream_start_in_lc(lc: &str, needle: &str) -> Option<usize> {
    let mut search_start = 0usize;
    while let Some(rel) = lc.get(search_start..)?.find(needle) {
        let abs = search_start + rel;
        search_start = abs + needle.len();

        if !is_command_position(lc.as_bytes(), abs) {
            continue;
        }
        let after_builtin_start = abs + needle.len();
        let after_builtin_raw = lc.get(after_builtin_start..)?;
        let leading_ws = after_builtin_raw.len() - after_builtin_raw.trim_start().len();
        let file_base = after_builtin_start + leading_ws;
        let after_builtin = lc.get(file_base..)?;
        let file_end_rel = after_builtin
            .find(|c: char| {
                c.is_whitespace() || c == ';' || c == '|' || c == '&' || c == '(' || c == '{'
            })
            .unwrap_or(after_builtin.len());
        if file_end_rel == 0 {
            continue;
        }
        let past_file = after_builtin.get(file_end_rel..)?;
        let past_redirects = skip_shell_redirects(past_file);
        let sep_len = if past_redirects.starts_with("&&") {
            2
        } else if past_redirects.starts_with(';') {
            1
        } else {
            continue;
        };
        // Byte offset of the downstream start within `lc`: file_base + bytes
        // consumed by the filename + redirects + separator, then skip whitespace.
        let consumed = {
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "past_redirects is a suffix of after_builtin (via skip_shell_redirects), so len difference is bounded and safe"
            )]
            {
                (after_builtin.len() - past_redirects.len()) + sep_len
            }
        };
        let downstream = past_redirects.get(sep_len..)?;
        let ws = downstream.len() - downstream.trim_start().len();
        if downstream.trim_start().is_empty() {
            continue;
        }
        return Some(file_base + consumed + ws);
    }
    None
}
