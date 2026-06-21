//! Inject the top-N mistake-ledger rows recorded by stop-gate violations.
//! SOURCE: arxiv.org/html/2512.11485 (Mistake Notebook Learning) — surface
//! the agent's own most-frequent failure modes BEFORE the turn starts so
//! behavioral self-correction can fire pre-action rather than post-block.
//! SOURCE: arxiv.org/pdf/2512.02389 — each row's title is already framed as
//! "BANNED [gate]: <sample> — INSTEAD: <fix>", avoiding parrot-the-mistake.
//!
//! The ledger lives under `pattern` category with key prefix `mistake.`
//! (no `STRICT_CATEGORIES` allowlist mutation). Falls back to None on any
//! RPC/parse failure — boot must NEVER block on memory-injection drift.
mod graph;
mod row;

use std::fmt::Write as _;

use row::fetch_mistake_row;

pub(in crate::gates::session_start) fn mistake_ledger_context() -> Option<String> {
    // Mistakes are GLOBAL (decision.mistakes-learnings-fully-global): read the
    // shared kavach-global namespace, never the session project — so a mistake
    // learned anywhere reinjects everywhere.
    let global = kavach_session::mistake_ledger::GLOBAL_NAMESPACE;
    // Primary: the graph anti_patterns the daemon embeds + clusters (the
    // autonomous loop). Closes the read/write split-brain — reinjection used to
    // read only the legacy `pattern` ledger below, never these nodes.
    if let Some(ctx) = graph::anti_pattern_context() {
        return Some(ctx);
    }
    // Fallback: the legacy `pattern`-category ledger (pre-graph rows, or rows the
    // capture path wrote when the graph RPC was down). Never block boot on drift.
    // 1. List candidate mistake.* keys from `kavach db query --category pattern`.
    let listing = std::process::Command::new("kavach")
        .args(["db", "query", "--project", global, "--category", "pattern"])
        .output()
        .ok()?;
    if !listing.status.success() {
        return None;
    }
    let stdout = String::from_utf8(listing.stdout).ok()?;
    let keys: Vec<String> = stdout.lines().filter_map(extract_mistake_key).collect();
    if keys.is_empty() {
        return None;
    }
    // 2. For each candidate, fetch full row, score with K-PRI (ledger weights).
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let scored: Vec<(String, String, f64)> = keys
        .into_iter()
        .filter_map(|k| {
            let r = fetch_mistake_row(global, &k)?;
            let sig = kavach_patterns::k_pri::Signals {
                hit_count: r.hit_count,
                #[expect(clippy::cast_precision_loss, reason="age_days is clamped to 86_400 buckets; f64 can represent all u64 div results")]
                age_days: now_unix.saturating_sub(r.last_seen_unix).saturating_div(86_400) as f64,
                ..Default::default()
            };
            let s = kavach_patterns::k_pri::score(sig, kavach_patterns::k_pri::W_MISTAKE_LEDGER);
            Some((r.title, k, s))
        })
        .collect();
    if scored.is_empty() {
        return None;
    }
    // 3. Rank by K-PRI descending; stable by key for tie-break.
    let mut ranked = scored;
    ranked.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    ranked.truncate(5);
    let mut ctx =
        String::from("\n[MISTAKE_LEDGER]\nstatus: anti-pattern reinforcement (K-PRI ranked)\n");
    for (title, _key, s) in &ranked {
        writeln!(ctx, "- [pri={s:.2}] {title}").ok();
    }
    ctx.push_str("rule: do NOT reproduce any BANNED phrase above; apply the INSTEAD: fix.\n");
    Some(ctx)
}

/// Extract a `mistake.<gate>.<sig>` key from a single `db query` listing line.
/// The CLI format puts the key at a stable token position; we scan tokens for
/// the prefix and return the first match.
fn extract_mistake_key(line: &str) -> Option<String> {
    line.split_whitespace()
        .find(|t| t.starts_with("mistake."))
        .map(str::to_owned)
}
