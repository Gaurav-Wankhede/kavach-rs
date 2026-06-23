//! Detects a BULK multi-file mutation typed inline as an ad-hoc shell command, and
//! steers it into ONE committed `scripts/<verb>.sh` driven by the Rust toolbelt
//! (rnr/sd/fd/rg). A rename / reference-rewrite / edge-case sweep across many files
//! must be one re-runnable, reviewable script — not N inline edits nor a one-shot
//! pipeline that vanishes from history. SOURCE: decision.bulk.one-script-not-n-edits;
//! toolbelt skill (rnr 0.5.1 batch-rename, sd 1.0.0 sed-replacement, fd 10.3.0).
//!
//! This is the BASH-side complement to `write_bypass` (which denies laundering ONE
//! Edit past the pre-write gate): here the concern is the opposite shape — a genuine
//! bulk op that should be authored ONCE as a script. Advisory tier; never blocks.

/// Bulk-content mutators: their presence in command position is a rewrite, not a read.
/// `rnr` is itself a batch renamer (any invocation is a bulk op). `sd`/`sed`/`perl -i`
/// are per-file rewriters that become bulk when fanned out (see `fanout_markers`).
const MUTATORS: &[&str] = &["rnr", "sd", "sed", "perl"];

/// Fan-out markers proving a mutator is applied across MANY targets in one command:
/// `fd … -x`, `xargs`, a `find … -exec`, or a glob/brace expansion of paths.
const FANOUT_MARKERS: &[&str] = &["-x ", " -exec", "xargs", "*.", "{}"];

/// Bulk-op steer vocabulary AS DATA: floor + additive graph overlay.
///
/// Mirrors [`crate::disobedience_guard::DisobedienceVocab`]: the compiled `const`
/// lists are the `Default` floor; a project's `gate.bulk_op_vocab` DB row ADDS
/// mutators / fan-out markers on top (research-refreshable, no rebuild). The graph
/// ADDS, never replaces — the floor always fires. `#[serde(default)]` degrades a
/// partial/malformed override to the full floor (fail-closed). SOURCE: decision.w5
/// (detector floor stays in-binary) + decision.bulk.one-script-not-n-edits.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct BulkOpVocab {
    /// Content/name mutators (command-position CLIs that rewrite files).
    pub mutators: Vec<String>,
    /// Markers proving a mutator is fanned out across many targets.
    pub fanout_markers: Vec<String>,
}

impl Default for BulkOpVocab {
    fn default() -> Self {
        Self {
            mutators: MUTATORS.iter().map(|s| (*s).to_owned()).collect(),
            fanout_markers: FANOUT_MARKERS.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

/// `Some(reason)` when the command is a bulk multi-file mutation that belongs in one
/// committed `scripts/*.sh`. Floor-default wrapper over [`detect_bulk_op_with`].
#[must_use]
pub fn detect_bulk_op(command: &str) -> Option<String> {
    detect_bulk_op_with(&BulkOpVocab::default(), command)
}

/// As [`detect_bulk_op`], but against a resolved [`BulkOpVocab`] (floor + overlay).
///
/// Fires when the command is a bulk MUTATION and is NOT already a committed script.
/// Bulk = a renamer (`rnr`) OR a per-file rewriter (`sd`/`sed -i`/`perl -i`) fanned
/// out across many targets (fan-out marker, OR ≥2 explicit path arguments). Carve:
/// a command that RUNS a `scripts/*.sh` is the sanctioned path and never fires.
#[must_use]
pub fn detect_bulk_op_with(vocab: &BulkOpVocab, command: &str) -> Option<String> {
    let lower = command.to_lowercase();
    // CARVE: the goal state — a single committed script — must never re-steer.
    if runs_committed_script(&lower) {
        return None;
    }
    let head = command_head(&lower);
    // `rnr` is a batch renamer: any invocation is inherently a bulk op (floor mutator).
    if head == "rnr" && vocab.mutators.iter().any(|m| m == "rnr") {
        return Some(steer("a batch rename (rnr)"));
    }
    // A per-file rewriter is bulk only when fanned out across many targets.
    let is_rewriter = vocab
        .mutators
        .iter()
        .any(|m| m != "rnr" && (head == m.as_str() || piped_to(&lower, m)));
    if !is_rewriter {
        return None;
    }
    let fanned = vocab.fanout_markers.iter().any(|f| lower.contains(f.as_str()))
        || explicit_path_args(command) >= 2;
    fanned.then(|| steer("a multi-file rewrite (sd/sed across many files)"))
}

/// The advisory text: name the op and point at the one-script canonical form.
fn steer(op: &str) -> String {
    format!(
        "{op} touches many files inline. Author it ONCE as a committed \
         `scripts/<verb>.sh` driven by the Rust toolbelt (rnr/sd/fd/rg), then run \
         `bash scripts/<verb>.sh` — one re-runnable, reviewable, git-tracked script \
         handles the rename + reference rewrite + edge cases atomically, instead of N \
         inline edits or a one-shot pipeline that leaves no artifact."
    )
}

/// True when the command's job is to RUN a `scripts/*.sh` file (the sanctioned path).
fn runs_committed_script(lower: &str) -> bool {
    lower.contains("scripts/") && lower.contains(".sh")
}

/// First word in command position (after stripping a leading `./`), lower-cased.
fn command_head(lower: &str) -> &str {
    let first = lower.split_whitespace().next().unwrap_or_default();
    first.strip_prefix("./").unwrap_or(first)
}

/// True when `tool` appears immediately after a pipe (`| sd`, `|sd`) — the
/// `rg -l | xargs sd` / `… | sd` fan-out shape where the mutator is not the head.
fn piped_to(lower: &str, tool: &str) -> bool {
    lower.contains(&format!("| {tool} ")) || lower.contains(&format!("|{tool} "))
}

/// Count explicit file-path arguments (tokens with a `/` or a source extension that
/// are not flags). ≥2 means the rewrite targets multiple files by hand.
fn explicit_path_args(command: &str) -> usize {
    command
        .split_whitespace()
        .filter(|t| !t.starts_with('-') && (t.contains('/') || has_source_ext(t)))
        .count()
}

/// True when a token ends in a tracked source extension.
fn has_source_ext(tok: &str) -> bool {
    const EXTS: &[&str] = &[".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".sql"];
    EXTS.iter().any(|e| tok.ends_with(e))
}

#[cfg(test)]
#[path = "bulk_op_guard_test.rs"]
mod tests;
