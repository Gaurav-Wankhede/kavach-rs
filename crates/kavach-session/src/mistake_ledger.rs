// MISTAKE LEDGER — observable record of every gate-block / behavioral violation.
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

/// Record a mistake into kavach-db under the `pattern` category.
///
/// Best-effort fire-and-forget: an unreachable server must NEVER block the
/// parent gate (the parent is itself a security gate). Returns the key written
/// so the caller can log it.
///
/// Key format: `mistake.<gate>.<sig8>` where sig8 = first 8 hex chars of
/// BLAKE3 over the banned sample, lowercased. Stable across runs so repeat
/// hits update the existing row (count++) instead of creating duplicates.
#[must_use = "if you ignore the returned key the gate cannot log the persisted row"]
pub fn record(m: &Mistake<'_>) -> String {
    if crate::mistake_ledger_graph::graph_path_enabled() {
        let session_id = crate::resolved_session_id();
        // Synchronous: the graph path is now an RPC round-trip to the server
        // (the single RocksDB writer), not a direct embedded-DB open — so no
        // tokio runtime is built here. SOURCE: rca.mistake-ledger-dark-via-direct-open.
        match crate::mistake_ledger_graph::try_record_via_graph(m, &session_id) {
            Ok(ids) => return ids,
            Err(e) => tracing::warn!(error = %e, "graph path failed, falling back to ledger"),
        }
    }
    let key = ledger_key(m.gate, m.banned_sample);

    // Probe the existing row so we can BUMP hit_count instead of overwriting
    // it back to 1. The K-PRI ranker (kavach_patterns::k_pri) reads hit_count
    // as the recurrence signal — losing it on every write breaks LFU ranking.
    // Global namespace, NOT m.project: a mistake is shared across all projects.
    let (prev_hits, exists) = read_hit_count(GLOBAL_NAMESPACE, &key);
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
    let dag = kavach_surreal::mistake_row_mermaid(m.gate, m.banned_sample, m.correct_action, new_hits);
    let content = format!(
        "anti-pattern row | gate={gate} turn={turn} hit_count={hits} last_seen_unix={ts} origin_project={origin}\n\
         ```mermaid\n{dag}```\n",
        gate = m.gate,
        turn = m.turn,
        hits = new_hits,
        ts = now_unix,
        origin = m.project,
    );

    // First hit → --new (CLI rejects --update-key on missing row). Subsequent
    // hits → --update-key. SOURCE: `kavach db write --help` strict-mode rules.
    let intent_flag = if exists { "--update-key" } else { "--new" };
    let intent_val = if exists { key.as_str() } else { "" };
    let mut args: Vec<&str> = vec![
        "db",
        "write",
        "--project",
        GLOBAL_NAMESPACE,
        "--category",
        "pattern",
        "--key",
        &key,
        "--title",
        &title,
        "--content",
        &content,
        intent_flag,
    ];
    if exists {
        args.push(intent_val);
    }
    // SOURCE: post_tool_algo_recorder.rs:301 — existing shellout pattern.
    // FIX [C1 reviewer cold-cluster] silent persistence failure broke K-PRI
    // contract: `let _ = output()` swallowed server-down / not-in-PATH errors
    // → hit_count never bumped → reinjection lost the recurrence signal.
    // SOURCE: github.com/rust-lang/rust/issues/73126 — output() error-handling
    // hazards; nonzero exit + nonempty stderr should never be silent.
    // Best-effort kept (no Result return; gate runs on every Stop), but
    // failures surface via tracing::warn so structured logging captures them.
    match Command::new("kavach").args(&args).output() {
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            tracing::warn!(
                key = %key,
                status = %o.status,
                stderr = %stderr,
                "record_mistake: db write failed"
            );
        }
        Err(e) => {
            tracing::warn!(
                key = %key,
                error = %e,
                "record_mistake: spawn failed"
            );
        }
        Ok(_) => {} // success — silent
    }
    key
}

/// Read the prior `hit_count` from an existing row's content. Returns (count, exists).
/// Best-effort: any failure ⇒ (0, false) — the next write becomes a `--new`.
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
    // but mark exists=true so we use --update-key (CLI would otherwise refuse).
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
mod tests {
    use super::*;

    #[test]
    fn ledger_key_is_stable_for_same_sample() {
        let a = ledger_key("permission", "should i proceed");
        let b = ledger_key("permission", "SHOULD I PROCEED");
        assert_eq!(a, b, "case-insensitive key for stable dedup");
    }

    #[test]
    fn ledger_key_differs_per_gate() {
        let a = ledger_key("permission", "should i proceed");
        let b = ledger_key("deferral", "should i proceed");
        assert_ne!(a, b);
    }

    #[test]
    fn ledger_key_sanitizes_gate_name() {
        let k = ledger_key("permission/seeking phase-2", "x");
        assert!(!k.contains('/'), "slashes stripped");
        assert!(!k.contains(' '), "spaces stripped");
        assert!(!k.contains('-'), "dashes stripped");
    }

    #[test]
    fn truncate_keeps_short_strings() {
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn truncate_caps_long_strings_with_ellipsis() {
        let out = truncate("0123456789ABCDEF", 5);
        assert!(out.ends_with('…'));
        assert!(out.chars().count() == 6);
    }
}
