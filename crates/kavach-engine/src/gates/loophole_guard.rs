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

/// Risk-bearing path markers — the scope half. Only content touching one of
/// these warrants the adversarial prompt; a docs/rename/format change does not.
const RISK_MARKERS: &[&str] = &[
    // authz / session
    "auth",
    "token",
    "session",
    "password",
    "permission",
    "authorize",
    // concurrency
    "lease",
    "lock",
    "mutex",
    "rwlock",
    "concurren",
    "atomic",
    "race",
    // money
    "payment",
    "balance",
    "transfer",
    // persistence
    "transaction",
    "persist",
    "commit",
    // secrets / crypto
    "secret",
    "encrypt",
    "decrypt",
    "nonce",
    "cipher",
    "hash",
    "hmac",
    "signature",
    // state machine
    "status",
    "state_transition",
    "claim",
    "acquire",
    // ssrf / outbound request
    "reqwest",
    "http_client",
    "fetch_url",
    "redirect",
    "webhook",
    "callback_url",
    // deserialization / parsing of untrusted input
    "deserialize",
    "from_str",
    "from_slice",
    "parse_json",
    "untrusted",
    // injection (sql / command / template)
    "sql",
    "query!",
    "execute(",
    "command::new",
    "shell",
    "render_template",
    // path traversal / file
    "path::new",
    "read_to_string",
    "open(",
    "join(",
    "canonicalize",
    // resource exhaustion / DoS
    "unbounded",
    "with_capacity",
    "loop {",
    "recursion",
    "read_to_end",
    // numeric truncation / overflow
    " as u",
    " as i",
    "wrapping_",
    "overflow",
    // information leakage
    "debug!(",
    "error!(",
    "{:?}",
    "to_string()",
];

/// Return the loophole self-interrogation advisory when `content` BOTH claims
/// completion AND touches a risk-bearing path. `None` otherwise — the common
/// case, so trivial work is never nagged.
pub(crate) fn check_loophole_interrogation(content: &str) -> Option<String> {
    if content.is_empty() {
        return None;
    }
    let lower = content.to_lowercase();
    let claims_done = DONE_PHRASES.iter().any(|p| lower.contains(p));
    if !claims_done {
        return None;
    }
    // Collect WHICH risk markers fired — that is the change's risk surface, which
    // steers the Brain-OS lens retrieval (multi-dimensional, not one frozen list).
    let fired: Vec<&str> = RISK_MARKERS
        .iter()
        .filter(|m| lower.contains(*m))
        .copied()
        .collect();
    if fired.is_empty() {
        return None;
    }
    let lens_list = lenses::lens_block(&fired);
    // RESOLVE, do NOT hand back: surface the change's risk surface + the lenses the
    // automated scan (`scan_changed_for_loopholes`) already runs over it. No CTA to
    // manually walk lenses or narrate a `Loopholes closed:` line — the lens scan
    // detects and records; the native triage agent fixes. This is awareness, not a
    // labor demand. (SOURCE: owner-gate/handback abolition — decision row.)
    Some(format!(
        "[LOOPHOLE_SURFACE] risk-bearing path touched. Relevant attack lenses for \
         this surface (the lens scan checks these automatically):\n{lens_list}"
    ))
}

/// Stop-gate variant: given the final assistant `message` of a turn, return the
/// terse loophole-surface awareness advisory when the turn claimed completion on a
/// risk-bearing path. `None` when the turn wrote no file or made no completion
/// claim.
///
/// RESOLVE, not handback: this NEVER halts and NEVER demands a `Loopholes closed:`
/// narration. The caller appends it as a clean-exit ride-along AND records a
/// ledger row; the automated lens scan (`scan_changed_for_loopholes`) + the native
/// triage agent do the actual closing. Awareness over labor-demand.
pub(crate) fn check_stop_interrogation(message: &str, wrote_this_turn: bool) -> Option<String> {
    // PRECISION GUARD: a loophole can only be LIVE if this turn actually WROTE a
    // risk-bearing path. Without this, the message-text trigger fires on a
    // read-only Q&A turn whose PROSE merely describes past risk fixes (words like
    // `lock`/`atomic`/`lease`/`done`) — a false-positive on a turn with no defect.
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
            sites.push(format!("{} {}:{} — {}", f.lens.slug(), path, f.line, f.hint));
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
        let line = format!("  … +{} more suspected site(s)\n", found.saturating_sub(shown));
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
