//! `CodeExecFlag` rows: tools allowlisted by NAME elsewhere (rg/find/git/go) but
//! carrying a flag that is a code-exec / arbitrary-file-write primitive.
//!
//! Closes the false-negative where a "safe" command name passes the name-based
//! allowlist yet the flag turns it into RCE.
//! SOURCE (verified 2026-06): <https://blog.trailofbits.com/2025/10/22/prompt-injection-to-rce-in-ai-agents/>
//!
//! Each regex anchors on the binary + a word-boundaried flag so benign
//! substrings (`--pretty`, `src/preprocessor.rs`, a path containing "pre") do
//! NOT match — the `(?:=|\s)\S` tail requires the flag to actually take a value.
use super::RawRow;
use crate::destructive_cli_guard::DestructiveCategory::CodeExecFlag as X;
use crate::destructive_cli_guard::DestructiveSeverity::P0Block;

pub(super) const ROWS: &[RawRow] = &[
    (
        X,
        P0Block,
        "ripgrep-pre",
        "rg --pre runs an arbitrary program on every searched file = RCE. Refuse; drop --pre.",
        r"(?i)\brg\s+(?:[^|;&]*\s)?--pre(?:=|\s)\S",
    ),
    (
        X,
        P0Block,
        "find-exec",
        "find -exec/-execdir/-delete runs commands on matched files. Refuse; use -print + a reviewed step.",
        r"(?i)\bfind\s+[^|;&]*\s-(?:exec(?:dir)?|delete)\b",
    ),
    (
        X,
        P0Block,
        "git-show-output",
        "git show/log --output= writes arbitrary bytes to an attacker-named path. Refuse.",
        r"(?i)\bgit\s+(?:show|log)\s+[^|;&]*--output(?:=|\s)\S",
    ),
    (
        X,
        P0Block,
        "go-test-exec",
        "go test -exec runs the test binary through an arbitrary program. Refuse.",
        r"(?i)\bgo\s+test\s+[^|;&]*-exec(?:=|\s)\S",
    ),
];
