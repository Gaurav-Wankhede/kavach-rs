//! Byte-level segment positioning: skipping leading shell redirects and deciding
//! whether a byte offset sits at command position (vs inside an argv flag).

/// Skip shell redirect tokens at the start of `s`, returning the remaining slice.
///
/// Handles `2>/dev/null`, `>/dev/null`, `2>&1`, `&>/dev/null`, `>file`, `2>file`,
/// `2>>file`, `>>file`. Stops at the first non-redirect token.
pub(crate) fn skip_shell_redirects(s: &str) -> &str {
    let mut cur = s.trim_start();
    loop {
        let before = cur;
        let after_op = if let Some(rest) = cur.strip_prefix("2>&") {
            rest
        } else if let Some(rest) = cur.strip_prefix("&>") {
            rest
        } else if let Some(rest) = cur.strip_prefix("2>>") {
            rest
        } else if let Some(rest) = cur.strip_prefix(">>") {
            rest
        } else if let Some(rest) = cur.strip_prefix("2>") {
            rest
        } else if let Some(rest) = cur.strip_prefix('>') {
            rest
        } else {
            break;
        };
        let target = after_op.trim_start();
        let end = target
            .find(|c: char| c.is_whitespace() || c == '&' || c == ';' || c == '|')
            .unwrap_or(target.len());
        let remaining = match target.get(end..) {
            Some(slice) => slice.trim_start(),
            None => break,
        };
        cur = remaining;
        if cur == before {
            break;
        }
    }
    cur
}

/// Determine whether `abs` is at command position in `bytes`.
///
/// Command position = start-of-input, or after a separator (`;`, `&`, `|`,
/// `(`, `{`, `\n`) possibly followed by whitespace. Used to distinguish
/// `source` builtin (command position) from `--source` argv flag.
pub(crate) fn is_command_position(bytes: &[u8], abs: usize) -> bool {
    if abs == 0 {
        return true;
    }
    let mut i = abs;
    while i > 0 {
        match bytes.get(i.saturating_sub(1)).copied() {
            Some(b' ' | b'\t') => {
                i = i.saturating_sub(1);
            }
            Some(b';' | b'&' | b'|' | b'(' | b'{' | b'\n') => return true,
            _ => return false,
        }
    }
    true
}
