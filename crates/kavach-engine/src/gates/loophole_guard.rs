//! Loophole self-interrogation prompt — the enforcement teeth behind the
//! `loophole_self_interrogation` directive in the global CLAUDE.md / Cursor rules.
//!
//! A loophole is a defect the happy path never exercises, so a clean build and a
//! green test suite do NOT prove its absence — only an adversarial question does.
//! This guard injects that question at the exact moment it matters: when written
//! content claims completion AND touches a risk-bearing path (auth / lease / lock
//! / money / persistence / concurrency / state transition). It is `P1Advisory`, NOT
//! a block — per the engine severity policy, a "did you think about loopholes?"
//! hard-block would false-positive on every trivial card. The model is reminded,
//! scoped to where it counts; it is never stopped.

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
    "auth",
    "lease",
    "lock",
    "mutex",
    "rwlock",
    "token",
    "session",
    "password",
    "secret",
    "payment",
    "balance",
    "transfer",
    "transaction",
    "persist",
    "commit",
    "concurren",
    "atomic",
    "race",
    "status",
    "state_transition",
    "claim",
    "acquire",
    "permission",
    "authorize",
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
    let touches_risk = RISK_MARKERS.iter().any(|m| lower.contains(m));
    if !touches_risk {
        return None;
    }
    Some(
        "[LOOPHOLE_CHECK]\n\
         This change claims completion on a risk-bearing path. A loophole found is \
         a loophole you FIX THIS TURN at its root — do NOT narrate it, do NOT defer \
         it, do NOT ship a summary in place of the fix.\n\
         RUN each lens. For every lens, the verdict is exactly one of:\n\
         - FIX NOW: write the guard/check at its root this turn, then cite file:line.\n\
         - FILE: out-of-scope only -> create a roadmap card + decision row naming \
         the exact failure mode (a parked loophole is tracked, never silent).\n\
         - N/A: prove it cannot occur and cite the file:line that defends against it.\n\
         The lenses:\n\
         - concurrency: two actors at once -> TOCTOU / lost-update / double-claim. \
         CLOSE with an atomic/compare-and-swap/lock, then cite it.\n\
         - failure: process dies mid-op -> orphaned lock / half-write / leaked task. \
         CLOSE with a guard/transaction/lease-expiry, then cite it.\n\
         - malformed: null/huge/wrong-type/hostile input -> panic / injection. \
         CLOSE by validating at the edge into a typed value, then cite it.\n\
         - authz: caller without rights -> missing check / confused-deputy / IDOR. \
         CLOSE by adding the check fail-closed, then cite it.\n\
         - replay: same request twice -> non-idempotent mutation. \
         CLOSE by making it idempotent, then cite it.\n\
         - boundary: empty / max / negative / off-by-one. \
         CLOSE by handling the bound, then cite it.\n\
         Emit a `Loopholes closed:` line: each lens -> FIXED at file:line, FILED as \
         <card-key>, or N/A at file:line. A `considered`/`noted`/`should` verdict \
         without a fix or a card is NOT acceptable — close it or file it, now."
            .into(),
    )
}

/// Marker the agent emits to show it CLOSED (not merely considered) the
/// loopholes. Matched case-insensitively; its presence satisfies the Stop-gate
/// check. Imperative on purpose: `closed` means each lens was fixed at `file:line`
/// or filed as a card — a passive `considered` line no longer satisfies the gate.
const ANSWERED_MARKER: &str = "loopholes closed";

/// Stop-gate variant: given the final assistant `message` of a turn, return the
/// loophole advisory when the turn claimed completion on a risk-bearing path but
/// emitted NO `Loopholes considered:` line. `None` otherwise — so a turn that
/// either did no risk work, made no completion claim, OR already answered the
/// self-interrogation exits clean.
///
/// This is the Stop-gate's teeth for the loophole directive: it does NOT halt
/// (per the "kill blocking, keep auto-continue" policy) — the caller appends the
/// result as a clean-exit ride-along advisory AND records a mistake-ledger row,
/// feeding the learning loop so the omission is seen over time.
pub(crate) fn check_stop_interrogation(message: &str, wrote_this_turn: bool) -> Option<String> {
    // PRECISION GUARD: a loophole can only be LIVE if this turn actually WROTE a
    // risk-bearing path. Without this, the message-text trigger fires on a
    // read-only Q&A turn whose PROSE merely describes past risk fixes (words like
    // `lock`/`atomic`/`lease`/`done`) — a false-positive refuse-stop with no real
    // defect. A turn that wrote no file cannot have shipped a live loophole.
    if !wrote_this_turn {
        return None;
    }
    let base = check_loophole_interrogation(message)?;
    // Already answered -> satisfied, no nudge.
    if message.to_lowercase().contains(ANSWERED_MARKER) {
        return None;
    }
    Some(format!(
        "{base}\n\
         [STOP] This turn shipped risk-bearing work without a `Loopholes closed:` \
         line — meaning a loophole may be live and unfixed RIGHT NOW. Do NOT stop. \
         Run the lenses on what you just shipped and CLOSE each one at its root this \
         turn (or FILE it as a card), then emit the `Loopholes closed:` line. \
         Recorded to the mistake ledger. Fixing beats documenting — fix it now."
    ))
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
