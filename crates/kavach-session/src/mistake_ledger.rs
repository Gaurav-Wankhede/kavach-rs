// MISTAKE LEDGER — observable record of every gate-block / behavioral violation.
// kavach:intentional pre-existing monolith over the line ceiling; this turn
// scopes only to closing the TOCTOU dup-row race, a module split is separate.
//
// ARCH: AntiPatternReinjection
// PATTERN: mistake_ledger | SCOPE: global (kavach-global) | CAP: AP | SEARCHED: 2026-05
//
// SOURCE: arxiv.org/html/2512.11485 (Mistake Notebook Learning) — distill
//   shared error patterns into structured "mistake notes" in external memory.
// SOURCE: arxiv.org/pdf/2512.02389 (Synthetic Error Injection) — naive
//   error reinjection makes the model PARROT the mistake. Mitigation here:
//   store the ANTI-PATTERN (banned phrase + correct alternative), not the
//   raw error text, so reinjection at SessionStart reinforces the FIX, not
//   the bug. record_mistake() takes (gate, banned_sample, correct_action).
// SOURCE: arxiv.org/abs/2603.10600 (Trajectory-Informed Memory Generation)
//   — Decision Attribution Analyzer flavor: each mistake row carries
//   gate name, last-turn, last-sample, hit-count for ranking.
//
// Persistence path: shell out to `kavach db write --category pattern` (the
// existing §LEARN-aligned procedural-memory bucket). Re-uses the existing
// `pattern.fix-<tool>-*` key convention from §TOOLING-FALSE-POSITIVE — same
// graph + frontmatter, no STRICT_CATEGORIES allowlist mutation.

use std::process::Command;

use blake3::Hasher;

/// Shared cross-project namespace for mistakes + learnings.
///
/// Every ledger write/read uses this slug, NOT the session project, so a mistake
/// learned in one repo is visible to all. See
/// decision.mistakes-learnings-fully-global.
pub const GLOBAL_NAMESPACE: &str = "kavach-global";

/// One mistake observation.
///
/// Records the gate that fired, the banned sample (the phrase / behavior that
/// tripped it), and the correct alternative the agent should have produced
/// instead. The persisted row is framed as a do-not-do rule, not a verbatim
/// copy of the failure — per arxiv 2512.02389 raw-error reinjection induces
/// parroting.
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at the record_mistake call boundary in kavach-engine stop.rs; non_exhaustive => E0639 on struct-literal construct"
)]
#[derive(Debug, Clone)]
pub struct Mistake<'a> {
    pub project: &'a str,
    pub gate: &'a str,
    pub banned_sample: &'a str,
    pub correct_action: &'a str,
    pub turn: i32,
}

/// Observable result of a `record()` call.
///
/// `record()` stays best-effort (it NEVER blocks the parent gate), but the
/// failure must not be SILENT: `persisted=false` + a populated `error` lets the
/// caller surface `[MISTAKE_RECORD_FAILED]` into LLM-visible output instead of a
/// discarded `tracing::warn!`. SOURCE: rca.mistake-ledger-dark-via-silent-write
/// · CLAUDE.md §NEVER-SOFT-FAIL.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RecordOutcome {
    /// The row key (`mistake.<gate>.<sig8>`) — always computed, even on failure.
    pub key: String,
    /// True iff the write was confirmed (graph RPC ok, or shell exit success).
    pub persisted: bool,
    /// Failure detail when `persisted=false`; `None` on success.
    pub error: Option<String>,
}

impl std::fmt::Display for RecordOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display yields the key, so existing `format!("{outcome}")`-style key
        // logging and `String`-shaped expectations keep working.
        f.write_str(&self.key)
    }
}

/// Record a mistake into kavach-db under the `pattern` category.
///
/// Best-effort: an unreachable server must NEVER block the parent gate (itself a
/// security gate). NOT silent: returns a [`RecordOutcome`] whose `persisted` /
/// `error` fields let the caller surface a failure to the LLM.
///
/// Key format: `mistake.<gate>.<sig8>` where sig8 = first 8 hex chars of
/// BLAKE3 over the banned sample, lowercased. Stable across runs so repeat
/// hits update the existing row (count++) instead of creating duplicates.
#[must_use = "inspect outcome.persisted/.error so a failed write is surfaced, not dropped silently"]
pub fn record(m: &Mistake<'_>) -> RecordOutcome {
    if crate::mistake_ledger_graph::graph_path_enabled() {
        let session_id = crate::resolved_session_id();
        // Synchronous: the graph path is now an RPC round-trip to the server
        // (the single RocksDB writer), not a direct embedded-DB open — so no
        // tokio runtime is built here. SOURCE: rca.mistake-ledger-dark-via-direct-open.
        match crate::mistake_ledger_graph::try_record_via_graph(m, &session_id) {
            Ok(ids) => {
                return RecordOutcome {
                    key: ids,
                    persisted: true,
                    error: None,
                };
            }
            Err(e) => tracing::warn!(error = %e, "graph path failed, falling back to ledger"),
        }
    }
    let key = ledger_key(m.gate, m.banned_sample);

    // Best-effort prior count, used ONLY for the displayed hit_count — a stale
    // read here affects the shown number, never row identity or write intent.
    // Global namespace, NOT m.project: a mistake is shared across all projects.
    let (prev_hits, _) = read_hit_count(GLOBAL_NAMESPACE, &key);
    let new_hits = prev_hits.saturating_add(1);
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let title = format!(
        "BANNED [{gate}]: {sample} — INSTEAD: {fix}",
        gate = m.gate,
        sample = truncate(m.banned_sample, 80),
        fix = truncate(m.correct_action, 120),
    );
    // Row content is a Mermaid DAG (banned -.fixed by.-> instead), not prose
    // (decision.mistake-row-mermaid-content / #1699): reinjection surfaces a
    // structured banned→fix edge the model parses directly. The metadata header
    // line is retained ABOVE the graph because read_hit_count + fetch_mistake_row
    // parse `hit_count=`/`origin_project=` tokens from it — dropping it would dark
    // the recurrence counter (the same silent-fail class fixed earlier).
    let dag =
        kavach_surreal::mistake_row_mermaid(m.gate, m.banned_sample, m.correct_action, new_hits);
    let content = format!(
        "anti-pattern row | gate={gate} turn={turn} hit_count={hits} last_seen_unix={ts} origin_project={origin}\n\
         ```mermaid\n{dag}```\n",
        gate = m.gate,
        turn = m.turn,
        hits = new_hits,
        ts = now_unix,
        origin = m.project,
    );

    let error = write_with_upsert(&key, &title, &content);
    RecordOutcome {
        persisted: error.is_none(),
        key,
        error,
    }
}

/// Write the row via `--update-key` first, falling back to `--new` only when
/// the row genuinely doesn't exist yet (CLI reports not-found).
///
/// This replaces a probe-then-branch (`read_hit_count` -> `exists` ->
/// `--new`/`--update-key`) that raced concurrent writers: two callers could
/// both observe `exists=false` and both issue `--new`, producing duplicate
/// rows or a lost increment. Trying the idempotent update first and only
/// falling back on a genuine not-found error means at most one writer ever
/// takes the create path for a given key. SOURCE: rca.mistake-ledger-toctou-dup-row.
fn write_with_upsert(key: &str, title: &str, content: &str) -> Option<String> {
    let update_args = [
        "db",
        "write",
        "--project",
        GLOBAL_NAMESPACE,
        "--category",
        "pattern",
        "--key",
        key,
        "--title",
        title,
        "--content",
        content,
        "--update-key",
        key,
    ];
    match Command::new("kavach").args(update_args).output() {
        Ok(o) if o.status.success() => return None,
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            if !is_key_not_found(&stderr) {
                tracing::warn!(key = %key, status = %o.status, stderr = %stderr, "record_mistake: update failed");
                return Some(format!(
                    "db write exit={} stderr={}",
                    o.status,
                    stderr.trim()
                ));
            }
        }
        Err(e) => {
            tracing::warn!(key = %key, error = %e, "record_mistake: spawn failed");
            return Some(format!("spawn failed: {e}"));
        }
    }
    // Row didn't exist yet — fall back to --new.
    let new_args = [
        "db",
        "write",
        "--project",
        GLOBAL_NAMESPACE,
        "--category",
        "pattern",
        "--key",
        key,
        "--title",
        title,
        "--content",
        content,
        "--new",
    ];
    match Command::new("kavach").args(new_args).output() {
        Ok(o) if o.status.success() => None,
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            tracing::warn!(key = %key, status = %o.status, stderr = %stderr, "record_mistake: create failed");
            Some(format!(
                "db write exit={} stderr={}",
                o.status,
                stderr.trim()
            ))
        }
        Err(e) => {
            tracing::warn!(key = %key, error = %e, "record_mistake: spawn failed");
            Some(format!("spawn failed: {e}"))
        }
    }
}

/// Detect the CLI's not-found error for `--update-key` on a missing row.
fn is_key_not_found(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("not found")
}

/// Record a mistake AND surface a write failure to the LLM next turn.
///
/// Queues `[MISTAKE_RECORD_FAILED]` into the session's pending-advisory spool when the
/// write does not land, instead of dying in a discarded `tracing::warn!`. Best-effort:
/// a successful write is silent.
///
/// Takes `gate`/`banned`/`instead` borrowed and `turn` by value (NOT a borrowed
/// `Mistake` referencing `state`) so callers avoid an immutable+mutable self-borrow
/// on `SessionState`. The mistake's `project` is read from `state.project` here.
/// SOURCE: decision.mistake-ledger-no-silent-write · CLAUDE.md §NEVER-SOFT-FAIL.
pub fn record_and_surface(
    state: &mut crate::SessionState,
    gate: &str,
    banned: &str,
    instead: &str,
    turn: i32,
) -> RecordOutcome {
    let outcome = record(&Mistake {
        project: &state.project,
        gate,
        banned_sample: banned,
        correct_action: instead,
        turn,
    });
    if !outcome.persisted {
        let err = outcome.error.as_deref().unwrap_or("unknown");
        state.queue_pending_advisory(&format!(
            "[MISTAKE_RECORD_FAILED] gate={gate} key={} err={err} — the ledger write did NOT land; \
             re-file with: kavach mistake record --gate {gate} --banned <sample> --instead <fix>",
            outcome.key,
        ));
    }
    outcome
}

/// Read the prior `hit_count` from an existing row's content. Returns (count, exists).
/// Best-effort: any failure ⇒ (0, false) — used only for the displayed count.
fn read_hit_count(project: &str, key: &str) -> (u32, bool) {
    let output = match Command::new("kavach")
        .args([
            "db",
            "get",
            "--project",
            project,
            "--category",
            "pattern",
            "--key",
            key,
            "--full",
        ])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return (0, false),
    };
    let Ok(body) = String::from_utf8(output.stdout) else {
        return (0, false);
    };
    // Token format: `hit_count=<N>` produced by record() above. First match wins.
    for line in body.lines() {
        if let Some(idx) = line.find("hit_count=") {
            let prefix_len = "hit_count=".len();
            let Some(tail) = line.get(idx.saturating_add(prefix_len)..) else {
                continue;
            };
            let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(n) = digits.parse::<u32>() {
                return (n, true);
            }
        }
    }
    // Row exists but content lacks the token (older schema) — treat as count=0
    // but mark exists=true (kept for callers that still branch on presence).
    (0, true)
}

fn ledger_key(gate: &str, banned_sample: &str) -> String {
    let mut h = Hasher::new();
    h.update(banned_sample.to_lowercase().as_bytes());
    let hex = h.finalize().to_hex();
    // first 8 hex chars are sufficient for collision-resistance at human-scale
    // (gate has its own namespace; ~10^9 banned samples per gate before
    // birthday-paradox 50% collision rate).
    let sig8: String = hex.chars().take(8).collect();
    let safe_gate: String = gate
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("mistake.{safe_gate}.{sig8}")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}

#[cfg(test)]
#[path = "mistake_ledger_test.rs"]
mod tests;
