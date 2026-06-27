//! Path-argument detection for `.env` targets — replaces the `lc.contains(".env")`
//! substring test that over-blocked repo searches like `rg PAT crates .env`.
//!
//! Arch: a `DotenvPathArgumentTest` (substring-overmatch false positive vs
//! filename-undermatch). Rejected `lc.contains(".env")` (matches `.env` inside dir
//! paths, search patterns, and longer tokens — the observed FP) and a full shfmt
//! AST (adds a Go-binary dep; a substring-free token scan suffices for the
//! read/grep classes). TIME: O(n) one pass over whitespace tokens | SPACE: O(1).
//! Tradeoff: token-basename match still can't see `$(...)`-built filenames; those
//! fall through to the generic secret-read classes. Acceptable: this module's job
//! is only the `.env`-FILE distinction, not all secrets.
//! SOURCE: <https://blog.trailofbits.com/2025/10/22/prompt-injection-to-rce-in-ai-agents/>
//! ("regex defenses are a cat-and-mouse game" — match the path argument, not a
//! substring); <https://github.com/AnswerDotAI/safecmd> (token walk).
//! Failure mode: a token whose basename is exactly `.env`/`.envrc`/`.env.<x>` is a
//! target; a real source dir among the path roots downgrades a multi-root search
//! to ALLOW (it is a repo grep, not a dotenv read).

/// True when `tok` (one shell word, already lowercased) names a `.env` file —
/// i.e. its **basename** is `.env`, `.envrc`, or `.env.<suffix>`.
///
/// Strips a leading `./` and any `=value` from `--file=.env`-style flags so the
/// basename test sees the path. A token like `crates/.env-fixtures` does NOT
/// match: its basename is `.env-fixtures`, not a dotenv filename.
fn token_is_dotenv_file(tok: &str) -> bool {
    // Drop an `--flag=` prefix so `--env-file=.env` is judged on `.env`.
    let val = tok.rsplit('=').next().unwrap_or(tok);
    let path = std::path::Path::new(val.trim_start_matches("./"));
    let Some(base) = path.file_name().and_then(|b| b.to_str()) else {
        return false;
    };
    base == ".env" || base == ".envrc" || base.starts_with(".env.")
}

/// Known source roots whose presence in a multi-path search proves the command
/// is a repository grep, not a dotenv read. See `decision.engine.dotenv_named_roots_only`.
fn is_source_root(tok: &str) -> bool {
    const ROOTS: &[&str] = &[
        "crates", "src", "tests", "lib", "app", "apps", "packages", "services", "core",
    ];
    let t = tok.trim_end_matches('/');
    ROOTS.contains(&t)
}

/// Decide whether a command actually targets a `.env` FILE for reading/searching.
///
/// Returns `false` (allow) when no token's basename is a dotenv file, OR when a
/// dotenv token co-occurs with a real source root (multi-root repo search — the
/// observed false positive `rg PAT crates .env`). Returns `true` only when a
/// dotenv file is a genuine target.
pub(crate) fn targets_dotenv_file(lc: &str) -> bool {
    let mut saw_dotenv = false;
    let mut saw_source_root = false;
    for tok in lc.split_whitespace() {
        if tok.starts_with('-') {
            // a flag like `--env-file=.env` still carries a value worth testing
            if token_is_dotenv_file(tok) {
                saw_dotenv = true;
            }
            continue;
        }
        if token_is_dotenv_file(tok) {
            saw_dotenv = true;
        } else if is_source_root(tok) {
            saw_source_root = true;
        }
    }
    saw_dotenv && !saw_source_root
}

#[cfg(test)]
#[path = "target_test.rs"]
#[cfg(test)]
#[path = "target_test.rs"]
mod tests;