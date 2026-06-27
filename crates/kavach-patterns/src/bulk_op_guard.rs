//! Detects a BULK multi-file mutation typed inline as an ad-hoc shell command.
//!
//! Steers it into ONE committed `scripts/<verb>.sh` driven by the Rust toolbelt
//! (rnr/sd/fd/rg). A rename / reference-rewrite / edge-case sweep across many files
//! must be one re-runnable, reviewable script — not N inline edits nor a one-shot
//! pipeline that vanishes from history. SOURCE: decision.bulk.one-script-not-n-edits;
//! toolbelt skill (rnr 0.5.1 batch-rename, sd 1.0.0 sed-replacement, fd 10.3.0).
//!
//! This is the BASH-side complement to `write_bypass` (which denies laundering ONE
//! Edit past the pre-write gate): here the concern is the opposite shape — a genuine
//! bulk op that should be authored ONCE as a script. Advisory tier; never blocks.
/// Bulk-content mutators: their presence in command position is a rewrite, not a read.
/// Rust-toolbelt-first: `rnr` (batch renamer — any invocation is a bulk op), `sg` /
/// `ast-grep` (structural multi-file rewrite — bulk by nature), and `sd` (the modern
/// `sed`) / `sed` / `perl -i`, which are per-file rewriters that become bulk when
/// fanned out (see `fanout_markers`). `rnr` / `sg` / `ast-grep` are inherent-bulk
/// (in `INHERENT_BULK`); the rest need a fan-out marker.
const MUTATORS: &[&str] = &["rnr", "sg", "ast-grep", "sd", "sed", "perl"];
/// Mutators whose every invocation is inherently a bulk op (a batch renamer / a
/// structural rewriter operate across a tree by design, not per single file).
const INHERENT_BULK: &[&str] = &["rnr", "sg", "ast-grep"];
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
    /// Mutators that are bulk on EVERY invocation (batch renamer / structural rewrite).
    pub inherent: Vec<String>,
    /// Markers proving a mutator is fanned out across many targets.
    pub fanout_markers: Vec<String>,
}
impl Default for BulkOpVocab {
    fn default() -> Self {
        Self {
            mutators: MUTATORS.iter().map(|s| (*s).to_owned()).collect(),
            inherent: INHERENT_BULK.iter().map(|s| (*s).to_owned()).collect(),
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
    // CARVE: the goal state — running the committed script (preferably via a `just`
    // recipe, else `bash scripts/*.sh`) — must never re-steer.
    if runs_committed_script(&lower) {
        return None;
    }
    let head = command_head(&lower);
    // Inherent-bulk mutators (rnr / sg / ast-grep): any invocation is a bulk op.
    if vocab.inherent.iter().any(|m| m == head) {
        return Some(steer(&format!("a batch / structural rewrite ({head})")));
    }
    // A per-file rewriter (sd/sed/perl) counts when it is the head, is piped to, or
    // is the target of a fan-out driver (`fd -x sd`, `xargs sd`, `find -exec sed`).
    let rewriters: Vec<&String> = vocab
        .mutators
        .iter()
        .filter(|m| !vocab.inherent.contains(*m))
        .collect();
    let has_rewriter = rewriters
        .iter()
        .any(|m| head == m.as_str() || word_present(&lower, m));
    if !has_rewriter {
        return None;
    }
    // Bulk = the rewriter is fanned out (fd -x / xargs / -exec / glob / {}), or it
    // names ≥2 explicit file paths by hand.
    let fanned = vocab
        .fanout_markers
        .iter()
        .any(|f| lower.contains(f.as_str()))
        || explicit_path_args(command) >= 2;
    // A lone `sd 'a' 'b' one.rs` (head rewriter, no fan-out) is NOT bulk — that is
    // the write-bypass guard's single-file domain; only steer when truly fanned out.
    fanned.then(|| steer("a multi-file rewrite (sd/sed across many files)"))
}
/// True when `tool` appears as a standalone word (driver target like `xargs sd` /
/// `fd … -x sd`), not as a substring of another token (`sediment`, `password`).
fn word_present(lower: &str, tool: &str) -> bool {
    lower.split_whitespace().any(|w| w == tool)
}
/// The advisory text: name the op and point at the one-script canonical form, run
/// via a `just` recipe (preferred) — itself wrapping the committed `scripts/<verb>.sh`.
fn steer(op: &str) -> String {
    format!(
        "{op} touches many files inline. Author it ONCE as a committed \
         `scripts/<verb>.sh` driven by the Rust toolbelt (rnr/sg/sd/fd/rg), expose it \
         as a `just <verb>` recipe, and run `just <verb>` — one re-runnable, \
         reviewable, git-tracked script handles the rename + reference rewrite + edge \
         cases atomically, instead of N inline edits or a one-shot pipeline that \
         leaves no artifact."
    )
}
/// True when the command's job is to RUN the committed bulk script: a `just` recipe
/// (the preferred entry point) OR a direct `scripts/*.sh` invocation.
fn runs_committed_script(lower: &str) -> bool {
    command_head(lower) == "just" || (lower.contains("scripts/") && lower.contains(".sh"))
}
/// First word in command position (after stripping a leading `./`), lower-cased.
fn command_head(lower: &str) -> &str {
    let first = lower.split_whitespace().next().unwrap_or_default();
    first.strip_prefix("./").unwrap_or(first)
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
