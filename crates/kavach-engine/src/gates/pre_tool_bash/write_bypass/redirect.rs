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
///
/// A `>` INSIDE a single- or double-quoted span is DATA, not a redirect operator
/// (e.g. the `<cmd>`/`a>b` text in a `kavach db write --content "..."` payload),
/// so quoted regions are skipped. A real `cmd > file` redirect lives outside
/// quotes and is still found. This is byte-scoped to the redirect detector;
/// capability signatures matched elsewhere (`open('f','w')`, `| psql`) keep
/// working inside quotes, where those launders genuinely live.
fn redirect_op_pos(part: &str) -> Option<usize> {
    let b = part.as_bytes();
    let mut i = 0;
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b.get(i).copied();
        // Track quote state; a `>` enclosed in quotes is an argument, not an op.
        if let Some(q) = quote {
            if c == Some(q) {
                quote = None;
            }
            i = i.saturating_add(1);
            continue;
        }
        if c == Some(b'\'') || c == Some(b'"') {
            quote = c;
            i = i.saturating_add(1);
            continue;
        }
        if c != Some(b'>') {
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

/// Split a compound command on `&&`/`||`/`;` separators that lie OUTSIDE any
/// quoted span, so a `;`/`&&` INSIDE a quoted argument (a `--content "...; ..."`
/// payload) does not shred the quote — keeping the quoted region intact as one
/// fragment lets `redirect_op_pos` mask its interior `>`. Bare `&` is NOT a
/// split point (it would cut `2>&1`); `redirect_op_pos` classifies fd-dup itself.
fn split_outside_quotes(lower: &str) -> Vec<&str> {
    let b = lower.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let mut quote: Option<u8> = None;
    // All separators/quotes are single-byte ASCII, so every `start`/`i` recorded
    // here lands on a UTF-8 char boundary. We still slice via `str::get` so a
    // multi-byte char in the payload can never panic (clippy `string_slice`).
    while let Some(&c) = b.get(i) {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            i = i.saturating_add(1);
            continue;
        }
        if c == b'\'' || c == b'"' {
            quote = Some(c);
            i = i.saturating_add(1);
            continue;
        }
        // Separators (outside quotes): `&&`, `||`, `;`.
        let two = b.get(i..i.saturating_add(2));
        if two == Some(b"&&".as_slice()) || two == Some(b"||".as_slice()) {
            if let Some(seg) = lower.get(start..i) {
                parts.push(seg);
            }
            i = i.saturating_add(2);
            start = i;
            continue;
        }
        if c == b';' {
            if let Some(seg) = lower.get(start..i) {
                parts.push(seg);
            }
            i = i.saturating_add(1);
            start = i;
            continue;
        }
        i = i.saturating_add(1);
    }
    if let Some(seg) = lower.get(start..) {
        parts.push(seg);
    }
    parts
}

/// Check each subcommand in a compound command for file redirects. Splits on
/// `&&`/`||`/`;` (outside quotes) so a safe redirect in one part cannot exempt a
/// dangerous one in another, while a separator INSIDE a quoted arg is preserved.
pub(super) fn has_file_redirect(lower: &str) -> bool {
    for part in split_outside_quotes(lower) {
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
