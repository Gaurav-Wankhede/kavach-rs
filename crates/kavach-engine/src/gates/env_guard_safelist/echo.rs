//! `echo`-expansion safety scan: true iff every `$VAR`/`${VAR}` reference names
//! a POSIX-safe system var (shell specials like `$$`/`$1`/`$?` are ignored).
use super::is_safe_system_var;

/// True when an `echo` command references ONLY safe POSIX system vars (or none).
///
/// Uses manual peek+next rather than `take_while`: `take_while` is lazy and
/// consumes one element past the predicate failure, which would swallow the
/// next `$` in `$HOME$SECRET`-style concatenations.
pub(crate) fn echo_only_references_safe_vars(lc: &str) -> bool {
    let mut chars = lc.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            continue;
        }
        let braced = matches!(chars.peek(), Some(&'{'));
        if braced {
            chars.next();
        }
        match chars.peek() {
            Some(&ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
            _ => continue,
        }
        let mut var = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                var.push(ch);
                chars.next();
            } else {
                break;
            }
        }
        if var.is_empty() {
            continue;
        }
        if !is_safe_system_var(&var) {
            return false;
        }
        if braced {
            skip_to_brace_close(&mut chars);
        }
    }
    true
}

/// Advance the iterator past a `${...}` body to its closing `}` (or next `$`).
fn skip_to_brace_close(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(&ch) = chars.peek() {
        if ch == '}' {
            chars.next();
            break;
        }
        if ch == '$' {
            break;
        }
        chars.next();
    }
}
