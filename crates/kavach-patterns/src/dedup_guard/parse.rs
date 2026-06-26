//! Line parsers for `dedup_guard`: extract the name a `use` binds and the name an
//! item definition introduces. Both are pure, allocation-free `&str` slicers so the
//! guard stays a linear scan with no regex dependency.

/// Item keywords that introduce a *new definition* of a name. A `use` of name `N`
/// followed by any of these defining `N` is a redefinition of an imported symbol.
const DEF_KEYWORDS: [&str; 5] = ["struct", "enum", "fn", "const", "static"];

/// Final segment a `use` line binds into scope:
/// `use core_utils::config::AppConfig;` -> `Some("AppConfig")`. Aliased imports
/// (`as X`) bind the alias. Grouped (`{A, B}`) and glob (`*`) imports are skipped —
/// no single bound name to shadow.
pub(super) fn imported_name(use_line: &str) -> Option<&str> {
    let body = use_line
        .trim()
        .strip_prefix("use ")?
        .split(';')
        .next()?
        .trim();
    if body.contains('{') || body.contains('*') {
        return None;
    }
    if let Some((_, alias)) = body.rsplit_once(" as ") {
        return Some(alias.trim());
    }
    body.rsplit("::")
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Name a definition line introduces, if the line starts an item whose keyword is
/// in `DEF_KEYWORDS`. Leading visibility/qualifier tokens (`pub`, `pub(crate)`,
/// `async`, `unsafe`, `const`, `extern`) are skipped to reach the keyword; a line
/// that is not an item start returns `None`.
pub(super) fn defined_name(line: &str) -> Option<&str> {
    let mut toks = line.split_whitespace();
    let mut saw_kw = false;
    for tok in toks.by_ref() {
        if DEF_KEYWORDS.contains(&tok) {
            saw_kw = true;
            break;
        }
        // A non-keyword, non-qualifier first token means this line is not an item
        // definition (a `let`, a call, a comment, an attribute, etc.).
        if !is_qualifier(tok) {
            return None;
        }
    }
    if !saw_kw {
        return None;
    }
    // The name runs from the token start up to the first non-identifier char —
    // `AppConfig()`, `Foo<T>`, `Bar{`, `BAZ:` all yield the bare ident. Rust idents
    // are `[A-Za-z0-9_]` (raw-ident `r#` prefix stripped first). `split` cuts on a
    // char predicate, so the slice is always a valid UTF-8 boundary.
    toks.next().and_then(|tok| {
        let ident = tok.strip_prefix("r#").unwrap_or(tok);
        let name = ident
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .unwrap_or_default();
        (!name.is_empty()).then_some(name)
    })
}

/// True for tokens that may legally precede an item keyword on a definition line.
fn is_qualifier(tok: &str) -> bool {
    matches!(
        tok,
        "pub" | "async" | "unsafe" | "const" | "extern" | "default"
    ) || tok.starts_with("pub(")
}
