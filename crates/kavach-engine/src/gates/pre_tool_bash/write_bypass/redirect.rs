//! File-redirect detection: locate a `>`/`>>` operator whose target is a path
//! (whitespace- and split-independent), excluding fd-duplication, numeric
//! comparisons, and operator glyphs. `/dev/null` + `/tmp/kavach` are safe sinks.

// FIX: [CWE-184 incomplete-denylist] redirect modeled as a whitespace-padded
// substring missed `echo data>file` (POSIX allows zero blanks around `>`).
// ROOT_CAUSE: redirect predicate narrower than the shell redirect grammar.
// SOLUTION: detect `>`/`>>` whitespace-independently; keep the `2>&1`,
// `/dev/null`, `/tmp/kavach`, and numeric-comparison exemptions.
// RESEARCH: https://cwe.mitre.org/data/definitions/184.html

/// Byte position of the first file-redirect operator (`>` or `>>`, optionally
/// fd-prefixed like `2>`) whose target is a path, or `None`. Classification is
/// by the redirect target rather than a fragile substring, so detection is both
/// whitespace-independent (`a>b` behaves like `a > b`) and split-independent.
/// Fd-duplication (`>&N`, `>& N`, `&>`) is not a file redirect because its
/// target is a descriptor; operator glyphs (`->`, `=>`, `>=`) and a `>`
/// followed by a digit (numeric comparison such as `len>80`) are excluded.
fn redirect_op_pos(part: &str) -> Option<usize> {
    let b = part.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b.get(i) != Some(&b'>') {
            i = i.saturating_add(1);
            continue;
        }
        let prev = (i > 0)
            .then(|| b.get(i.saturating_sub(1)).copied())
            .flatten();
        // `->`, `=>`, `>=` are operator glyphs, never a file redirect.
        if matches!(prev, Some(b'-' | b'=')) || b.get(i.saturating_add(1)) == Some(&b'=') {
            i = i.saturating_add(1);
            continue;
        }
        // Consume the append form `>>`.
        let after = if b.get(i.saturating_add(1)) == Some(&b'>') {
            i.saturating_add(2)
        } else {
            i.saturating_add(1)
        };
        // Skip optional spaces before the target.
        let mut j = after;
        while b.get(j) == Some(&b' ') {
            j = j.saturating_add(1);
        }
        match b.get(j) {
            // Numeric comparison: `>` then a digit — not a redirect.
            Some(b'0'..=b'9') => {
                i = after;
                continue;
            }
            // fd-duplication `>&1`/`&>`: target is a descriptor, not a file.
            Some(b'&') => {
                i = j.saturating_add(1);
                continue;
            }
            _ => {}
        }
        // Anything else (incl. `&>file` Bash all-streams redirect to a path).
        return Some(i);
    }
    None
}

/// Check each subcommand in a compound command for file redirects. Splits on
/// `&&`/`||`/`;` so a safe redirect in one part cannot exempt a dangerous one
/// in another. Does NOT split on bare `&` — that would shred `2>&1`;
/// `redirect_op_pos` already classifies fd-dup vs file by target.
pub(super) fn has_file_redirect(lower: &str) -> bool {
    let parts = lower
        .split("&&")
        .flat_map(|p| p.split("||"))
        .flat_map(|p| p.split(';'));
    for part in parts {
        let p = part.trim();
        let Some(pos) = redirect_op_pos(p) else {
            continue;
        };
        // Safe sinks evaluated from the redirect operator onward so a safe sink
        // elsewhere in the part cannot mask a dangerous one.
        let tail = p.get(pos..).unwrap_or("");
        let safe = tail.contains("/dev/null") || tail.contains("/tmp/kavach");
        if !safe {
            return true;
        }
    }
    false
}
