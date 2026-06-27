//! Legacy-tool gate: §TOOLBELT promoted from advisory to LAW.
//!
//! Detects bare POSIX tools (`grep`/`cat`/`find`/`sed`/...) that have a
//! mandated Rust toolbelt replacement and returns a `Hit` so the caller can
//! HARD-BLOCK (P0). `pre_tool_bash` wires this with `exit_pre_tool_deny` —
//! empirically verified to genuinely CANCEL Bash on this Claude Code
//! version (the existing psql/sqlx P0s use the same call; a live
//! `psql --version` was blocked, not merely warned).
//!
//! Matcher (research-grounded).
//! SOURCE: <https://github.com/openclaw/openclaw/issues/59600> — naive
//! first-word/substring allowlisting is exploitable for compound commands.
//! SOURCE: <https://github.com/anthropics/claude-code/issues/6409> — an
//! over-aggressive grep→rg hook can brick the CLI, so this matcher is
//! conservative and FAIL-CLOSED-AS-NO-BLOCK on parse ambiguity.
//! SOURCE: <https://crates.io/crates/shell-words> — POSIX.1-2008 split.
//!  1. `shell_words::split` → quote-aware tokens (a tool name inside a
//!     quoted arg, e.g. `-m "use grep"`, is DATA → never a hit).
//!  2. Split tokens on `|` `|&` `;` `&&` `||` `&` → validate EVERY pipeline
//!     segment's command word (a whole-string first-word check misses
//!     `git log | grep x` AND wrongly flags `ps aux | grep x`).
//!  3. Per segment: skip leading `!`, `time`, and `VAR=value` prefixes.
//!  4. `git`/known wrapper bin ⇒ EXEMPT (`git grep`/`cat-file`/`diff` are
//!     subcommands, not the POSIX bin).
//!  5. `find` ACTION mode (`-delete`/`-exec`/...) ⇒ EXEMPT (fd can't
//!     express these). `cat <<`/`cat <<-` heredoc ⇒ EXEMPT.
//!  6. Command word ∈ legacy set ⇒ Hit with the toolbelt replacement.
//!  7. Command substitution `$(` / backtick / process-sub `<(`/`>(` or a
//!     `shell_words::split` Err ⇒ return None. QUALITY gate, not the
//!     safety boundary (`prod_guard/env_guard` P0s precede it and own
//!     destructive/injection content). A missed lazy tool is recoverable;
//!     a false P0 with no escape is the worse failure (#6409).
/// A blocked legacy-tool invocation and its mandated replacement.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The legacy command word that triggered the block (e.g. `grep`).
    pub tool: String,
    /// The toolbelt replacement to use instead (e.g. `rg`).
    pub replacement: &'static str,
}
/// legacy command word → toolbelt replacement (§TOOLBELT canonical map).
fn replacement_for(cmd: &str) -> Option<&'static str> {
    Some(match cmd {
        "grep" | "egrep" | "fgrep" => "rg",
        "find" => "fd",
        "cat" => "bat",
        "sed" => "sd",
        "ls" => "eza",
        "jq" => "jaq",
        "curl" => "xh",
        "du" => "dust",
        "tree" => "erd",
        "ps" => "procs",
        "diff" => "difft",
        _ => return None,
    })
}
/// Tokens that, anywhere in a `find` segment, mean find is being used as an
/// ACTION (not a search) — `fd` cannot express these, so exempt.
const FIND_ACTION_TOKENS: &[&str] = &[
    "-delete", "-exec", "-execdir", "-ok", "-okdir", "-newer", "-prune", "-fprint",
];
/// Wrapper binaries whose first token is the bin and a later token is a
/// subcommand — the POSIX legacy tool is NOT being invoked.
fn is_wrapper_bin(cmd: &str) -> bool {
    matches!(cmd, "git" | "cargo" | "rustup" | "docker" | "kubectl")
}
/// True if `command` contains a construct that could let an argument escape
/// into command position (command/process substitution). Fail-closed: we
/// do NOT block on these (a different gate owns injection); we just decline
/// to assert a toolbelt Hit on an ambiguous parse.
fn has_ambiguous_substitution(command: &str) -> bool {
    command.contains("$(")
        || command.contains('`')
        || command.contains("<(")
        || command.contains(">(")
}
/// The command word of a pipeline segment: first token after skipping
/// `!`, `time`, and `VAR=value` assignment prefixes. None if the segment
/// has no command word (empty / all-prefixes).
fn segment_command_word(tokens: &[String]) -> Option<&str> {
    for tok in tokens {
        if tok == "!" || tok == "time" {
            continue;
        }
        // `VAR=value` assignment prefix: `=` before any `/`, with a
        // non-empty identifier LHS.
        if let Some(eq) = tok.find('=')
            && let Some(lhs) = tok.get(..eq)
        {
            let is_assignment = !lhs.is_empty()
                && !lhs.contains('/')
                && lhs.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
            if is_assignment {
                continue;
            }
        }
        return Some(tok.as_str());
    }
    None
}
/// Inspect a Bash command string. Returns `Some(Hit)` iff a pipeline
/// segment's command word is a bare legacy tool with a toolbelt
/// replacement and no exemption applies. Pure; no I/O.
#[must_use]
pub fn inspect(command: &str) -> Option<Hit> {
    let command = command.trim();
    if command.is_empty() || has_ambiguous_substitution(command) {
        return None; // fail-closed-as-no-block (see module docs)
    }
    // `cat <<EOF` / `cat <<-EOF` heredoc: stream construction, not a read.
    if command.contains("<<") {
        return None;
    }
    let Ok(tokens) = shell_words::split(command) else {
        return None; // unparseable → not our call to block
    };
    // Split the flat token stream into pipeline/sequence segments on the
    // shell operators (shell_words yields operators as standalone tokens).
    for segment in tokens.split(|t| matches!(t.as_str(), "|" | "|&" | ";" | "&&" | "||" | "&")) {
        let Some(cmd) = segment_command_word(segment) else {
            continue;
        };
        // `grep` and `/usr/bin/grep` and `./grep` all invoke the POSIX
        // tool — compare on the basename.
        let base = cmd.rsplit('/').next().unwrap_or(cmd);
        if is_wrapper_bin(base) {
            continue; // `git grep` / `cargo …` — subcommand, not POSIX bin
        }
        let Some(replacement) = replacement_for(base) else {
            continue;
        };
        // `ls` is only a hit when recursive (`-R`/`-r`) — plain `ls` is
        // ubiquitous; the `eza` swap there is noise, not a real concern.
        if base == "ls"
            && !segment
                .iter()
                .any(|t| t == "-R" || t == "-r" || t == "--recursive")
        {
            continue;
        }
        // `find` as an ACTION (delete/exec/...) — fd can't express it.
        if base == "find"
            && segment
                .iter()
                .any(|t| FIND_ACTION_TOKENS.contains(&t.as_str()))
        {
            continue;
        }
        return Some(Hit {
            tool: base.to_owned(),
            replacement,
        });
    }
    None
}
#[cfg(test)]
#[path = "legacy_tool_guard_tests.rs"]
mod tests;
