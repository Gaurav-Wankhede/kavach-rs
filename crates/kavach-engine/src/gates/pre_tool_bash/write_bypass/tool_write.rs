//! Detect file-writing CLI tools in command position whose invocation
//! designates an output path (creating/overwriting a file with no shell
//! redirect): `wget -O`, `curl -o`/`-O`, and `cp`/`mv`/`install`/`dd`/`truncate`.

use super::segment::{segment_first_word_is, segment_has_flag};

// Detect file-writing tools: wget, curl, cp, dd, mv, install with output flags.
// See decision.engine.tool_write_detection.

/// True when a file-writing tool in command position designates an output path.
/// Read-only/stdout forms (`curl URL`, `wget -qO-`, `--help`) and the
/// `/dev/null`/`/tmp/kavach`/kavach-binary safe sinks do not match.
///
/// EXEMPT: `~/.local/bin/kavach` — the kavach binary deployment path (§DEPLOY:
/// self-deploy needs cp + codesign; blocking it deadlocks deployment). On Apple
/// Silicon cp breaks the adhoc signature and codesign must re-sign.
/// RESEARCH: <https://github.com/Homebrew/brew/issues/9082>
pub(super) fn writes_via_tool(lower: &str) -> bool {
    // Safe sinks: /dev/null, kavach scratch dir, and the kavach binary path
    // (both expanded /Users/.../local/bin/kavach and unexpanded ~ forms).
    let safe_sink = lower.contains("/dev/null")
        || lower.contains("/tmp/kavach")
        || lower.contains("/.local/bin/kavach")
        || lower.contains("~/.local/bin/kavach");
    if safe_sink {
        return false;
    }
    // wget/curl only write a file when an output flag names one. Bare
    // `-O`/`-o` followed by `-` (stdout) or absent ⇒ not a file write.
    if segment_first_word_is(lower, "wget") {
        if lower.contains("--output-document=") && !lower.contains("--output-document=-") {
            return true;
        }
        if (lower.contains(" -o ") || lower.contains(" --output ") || lower.contains(" -o-"))
            && !lower.contains("o- ")
            && !lower.ends_with("o-")
        {
            return true;
        }
    }
    if segment_first_word_is(lower, "curl")
        && (lower.contains(" -o ")
            || lower.contains(" --output ")
            || lower.contains(" -o-")
            || segment_has_flag(lower, "curl", "-o"))
        && !lower.contains("-o- ")
        && !lower.ends_with("-o-")
    {
        return true;
    }
    // copy/move/overwrite utilities: their non-flag operand IS a file path.
    // Introspection flags (`--help`/`-h`/`--version`) short-circuit first.
    let introspection = lower.contains(" --help")
        || lower.ends_with(" -h")
        || lower.contains(" -h ")
        || lower.contains(" --version")
        || lower.ends_with(" --version");
    if !introspection {
        for tool in ["cp", "mv", "install", "dd", "truncate"] {
            if segment_first_word_is(lower, tool) {
                return true;
            }
        }
    }
    false
}
