//! Loophole-surface awareness on completion-claims over risk-bearing paths.
//! P1 advisory only, never a block. SOURCE: decision.loophole.resolve-not-handback.
mod lenses;
/// Completion-claim phrases — the trigger half. Mirrors `completion_guard` but
/// kept local so the two guards stay independently tunable.
const DONE_PHRASES: &[&str] = &[
    "done",
    "complete",
    "finished",
    "shipped",
    "implemented",
    "fixed",
    "verified",
    "works now",
    "ready",
];
/// Return the loophole self-interrogation advisory when `content` BOTH claims
/// completion AND touches a risk-bearing path. `None` otherwise — the common
/// case, so trivial work is never nagged.
///
/// Risk markers + dimension taxonomy are now TECH-AGNOSTIC and graph-sourced: the
/// `gate.loophole_vocab` overlay ADDS to the compiled cross-language floor (fail-
/// closed). SOURCE: decision.loophole-mistake-umbrella. Resolved against the active
/// session's project so each stack tunes its own markers without a rebuild.
pub(crate) fn check_loophole_interrogation(content: &str) -> Option<String> {
    if content.is_empty() {
        return None;
    }
    let project = kavach_session::get_or_create_session().project;
    let vocab = crate::gates::stop_dispatch::loophole_vocab_for(&project);
    check_loophole_with(&vocab, content)
}
/// As [`check_loophole_interrogation`] but against a resolved vocab (testable,
/// no session/RPC). The trigger markers + dimension labels come from the vocab,
/// not a frozen const table.
pub(crate) fn check_loophole_with(
    vocab: &kavach_patterns::loophole_vocab::LoopholeVocab,
    content: &str,
) -> Option<String> {
    let lower = content.to_lowercase();
    let claims_done = DONE_PHRASES.iter().any(|p| lower.contains(p));
    if !claims_done {
        return None;
    }
    // Collect WHICH trigger markers fired — the change's risk surface. Markers are
    // matched lower-cased so the agnostic floor (mixed-case across languages) fires.
    let fired: Vec<&str> = vocab
        .trigger_markers()
        .into_iter()
        .filter(|m| lower.contains(&m.to_lowercase()))
        .collect();
    if fired.is_empty() {
        return None;
    }
    let lens_list = lenses::lens_block(&fired);
    let dims = kavach_patterns::loophole_vocab::fired_dimensions(vocab, &fired);
    Some(format!(
        "[LOOPHOLE_SURFACE] risk-bearing path touched — fired dimensions: {dims}. \
         Lenses for these dimensions (the lens scan checks them automatically):\n{lens_list}"
    ))
}
/// Stop-gate variant: terse surface advisory when the turn claimed completion on a
/// risk path. Never halts. `None` when no file was written or no completion claim.
pub(crate) fn check_stop_interrogation(message: &str, wrote_this_turn: bool) -> Option<String> {
    // A loophole is only live if this turn WROTE — guards read-only prose-trigger FPs.
    if !wrote_this_turn {
        return None;
    }
    check_loophole_interrogation(message)
}
/// Max changed files scanned per Stop — the boundary brake so a huge multi-file
/// turn cannot stall the hot Stop path. Excess is named, never silently dropped.
const MAX_FILES_SCANNED: usize = 24;
/// Max suspected sites listed in the advisory — keeps the injected text bounded.
const MAX_SITES_LISTED: usize = 12;
/// Collect this turn's changed, scannable Rust files as `(path, content)` pairs,
/// bounded to `MAX_FILES_SCANNED`. Source = `git diff --name-only HEAD` (working-
/// tree changes since the last commit) — the same git-tracked view the CLI sweep
/// uses. Consistent with the existing Stop-path process I/O (`cargo check` in
/// `stop_dispatch/verify.rs`); the Stop gate is end-of-turn, not latency-critical.
/// Returns an owned Vec so the borrow of file contents is self-contained. Any git
/// or read error yields an empty Vec — detection is best-effort, never fatal.
pub(crate) fn changed_rust_files() -> Vec<(String, String)> {
    let Ok(out) = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|p| is_scannable_rust(p))
        .take(MAX_FILES_SCANNED)
        .filter_map(|p| std::fs::read_to_string(p).ok().map(|c| (p.to_owned(), c)))
        .collect()
}
/// True for a non-test Rust source — mirrors the CLI sweep's exclusion so the
/// gate and the sweep agree on scope. Test code (`tests.rs`, `*_test.rs`,
/// `*_tests.rs`, anything under a `tests/` dir) is excluded: it legitimately uses
/// `unwrap`/`expect`/index, so flagging it floods the advisory with non-defects.
fn is_scannable_rust(path: &str) -> bool {
    let p = std::path::Path::new(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("rs")) {
        return false;
    }
    let s = path.replace('\\', "/");
    if s.contains("/tests/") || s.starts_with("tests/") {
        return false;
    }
    p.file_name().and_then(|n| n.to_str()).is_some_and(|name| {
        // Mirror the CLI sweep: exclude tests.rs and any stem carrying a `_test`
        // segment (foo_test.rs / foo_tests.rs / foo_test_menu.rs sibling support).
        name != "tests.rs" && !name.trim_end_matches(".rs").contains("_test")
    })
}
/// Bounded loophole DETECTION over the turn's changed files — the M4 teeth that
/// turn the prompt-only self-interrogation into an actual lens scan. PURE: the
/// caller (the hook layer, which may touch git/fs) supplies `(path, content)`
/// pairs; this runs the shared `kavach_patterns::loophole_lens` kernel and returns
/// a concrete `[LOOPHOLE_SITES]` advisory naming suspected `(lens, file:line)`
/// hints, or `None` when nothing fired. Bounded by `MAX_FILES_SCANNED` (the hot
/// Stop path must never stall) and `MAX_SITES_LISTED`. Kavach DETECTS + RECORDS
/// only — the native subscription agent triages + fixes each site; no LLM here.
pub(crate) fn scan_changed_for_loopholes(files: &[(&str, &str)]) -> Option<String> {
    if files.is_empty() {
        return None;
    }
    let total_files = files.len();
    let scanned = total_files.min(MAX_FILES_SCANNED);
    let mut sites: Vec<String> = Vec::new();
    for (path, content) in files.iter().take(scanned) {
        for f in kavach_patterns::loophole_lens::scan_text(content) {
            sites.push(format!(
                "{} {}:{} — {}",
                f.lens.slug(),
                path,
                f.line,
                f.hint
            ));
        }
    }
    if sites.is_empty() {
        return None;
    }
    let found = sites.len();
    let shown = found.min(MAX_SITES_LISTED);
    let mut out = String::from(
        "[LOOPHOLE_SITES] bounded lens scan of this turn's changed files flagged \
         suspected loopholes (heuristic hints, not proof). Triage each via its \
         attack lens; FIX at the source or prove N/A with a file:line citation:\n",
    );
    for s in sites.iter().take(shown) {
        out.push_str("  - ");
        out.push_str(s);
        out.push('\n');
    }
    // No silent cap: name what was dropped (sites and/or files). Format into a
    // local first, then `push_str` — `String::push_str` is infallible, sidestepping
    // both the `push_str(&format!())` and the `fmt::Result` discard lints.
    if found > shown {
        let line = format!(
            "  … +{} more suspected site(s)\n",
            found.saturating_sub(shown)
        );
        out.push_str(&line);
    }
    if total_files > scanned {
        let line = format!(
            "  (scanned {scanned}/{total_files} changed files this turn; rerun `kavach loophole sweep` for the rest)\n"
        );
        out.push_str(&line);
    }
    Some(out)
}
#[cfg(test)]
#[path = "loophole_guard_tests.rs"]
mod tests;
