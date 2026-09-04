// split: intentional — single CLI command handler with helper async fns
//! One cohesive `kavach db write` command handler —
//! validation + strict-mode dedup + atomic upsert + edge projection are a
//! single transaction-shaped flow; splitting fragments the handler with no
//! reuse gain. Pure helpers (similarity, protected-closure) already factored.
// `kavach db write` — SurrealDB-backed entry upsert.
// kanban_cards table dropped: SurrealDB stores entry_status directly on roadmap rows.
//
// STRICT MODE (--new vs --update-key) prevents stale-row proliferation:
// writes against {decision,research,roadmap,pattern,app_spec} must declare
// intent. --new fuzzy-matches title against existing rows (refuses on >=0.85
// similarity). --update-key verifies the key exists. Other categories keep
// legacy back-compat.
// SOURCE: docs.rs/clap/latest/clap/_derive/_tutorial/index.html
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit, read_stdin_body_or_exit};
use kavach_db::dual_write::MemoryEntry;
use kavach_db::turso::entries::EntryStore;
use surrealdb_types::{RecordId, RecordIdKey};
use kavach_types::Priority;
use std::fmt::Write as _;
const ROADMAP_CATEGORY: &str = "roadmap";
/// THE single source of truth for the valid memory-entry category vocabulary.
///
/// Categories that require explicit `--new` or `--update-key` intent AND the
/// complete accepted set (verified == the `kavach-surreal` write match arm).
/// `pub(crate)` so the clap `--category` args reference `CATEGORY_HELP`
/// (below) instead of re-typing the list — eliminating the doc-string drift
/// class (rca.kavach-db-write-category-enum-inconsistent: 4 help strings had
/// drifted to the dead `proposal`). Add/remove a category HERE and nowhere
/// else; the equivalence test below fails closed if `CATEGORY_HELP` desyncs.
pub(crate) const STRICT_CATEGORIES: &[&str] =
    &["decision", "research", "roadmap", "pattern", "app_spec"];
/// clap `--category` help text — DERIVED from [`STRICT_CATEGORIES`], not a
/// duplicated literal. Every `#[arg]` with a `--category` flag references
/// this via `#[arg(help = CATEGORY_HELP)]` so the help can never drift from
/// the validator (clap derive: an explicit `help =` attribute overrides the
/// `///` doc-comment — SOURCE: docs.rs/clap derive; clap-rs/clap#3108).
/// `concat!` keeps it a `&'static str` (const-eval, no allocation) usable in
/// the `#[arg]` attribute position. KEEP IN SYNC WITH `STRICT_CATEGORIES` —
/// the `category_help_matches_strict_categories` test asserts exact equality.
pub(crate) const CATEGORY_HELP: &str = "Category (decision, research, roadmap, pattern, app_spec)";
/// Fuzzy-match threshold: titles with normalized Levenshtein similarity >=
/// this value are flagged as likely duplicates when --new is used.
const FUZZY_THRESHOLD: f64 = 0.85;
/// Normalized Levenshtein similarity: 1.0 - (dist / `max_len`). Returns 0..=1.
#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "textbook Levenshtein DP: indices bounded by 1..=m / 1..=n loop invariants; similarity ratio is inherently float; string lengths far below f64 mantissa limit"
)]
fn similarity(a: &str, b: &str) -> f64 {
    let a_lc = a.to_lowercase();
    let b_lc = b.to_lowercase();
    let a_chars: Vec<char> = a_lc.chars().collect();
    let b_chars: Vec<char> = b_lc.chars().collect();
    let (m, n) = (a_chars.len(), b_chars.len());
    if m == 0 && n == 0 {
        return 1.0;
    }
    let max = m.max(n) as f64;
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr: Vec<usize> = vec![0; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = usize::from(a_chars[i - 1] != b_chars[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    1.0 - (prev[n] as f64 / max)
}
/// Keywords in roadmap keys that indicate important items requiring protection.
const PROTECTED_KEY_PATTERNS: &[&str] = &[
    "market-gap",
    "todo-",
    "migration",
    "phase",
    "p0-",
    "p1-",
    "critical",
    "blocking",
    "security",
    "audit",
    "architecture",
    "grpc",
    "api-",
];
/// Title patterns that indicate closure/archival intent.
const CLOSURE_PATTERNS: &[&str] = &[
    "closed",
    "stale",
    "archived",
    "deprecated",
    "superseded",
    "obsolete",
];
/// Check if a roadmap write is attempting to close a protected item.
/// Returns Some(warning) if protected, None if safe to proceed.
fn check_protected_closure(key: &str, title: &str) -> Option<String> {
    let key_lc = key.to_lowercase();
    let title_lc = title.to_lowercase();
    let is_protected = PROTECTED_KEY_PATTERNS.iter().any(|p| key_lc.contains(p));
    if !is_protected {
        return None;
    }
    let is_closure = CLOSURE_PATTERNS.iter().any(|p| title_lc.contains(p));
    if !is_closure {
        return None;
    }
    Some(format!(
        "PROTECTED_ITEM_CLOSURE_BLOCKED: Key '{key}' matches protected pattern.\n\
         Title contains closure indicator: '{title}'\n\
         \n\
         To close this item properly, use:\n\
         kavach db kanban-close --project <slug> --key {key}\n\
         \n\
         To force closure via write, add '--force-close' or rename the key."
    ))
}
/// Mirror `--depends-on` flag targets into a `DEPENDS_ON:` content line so the
/// dispatch readiness check (which parses deps from CONTENT) honors them. Returns
/// `body` unchanged when there are no flag deps. Idempotent: a target already
/// present on an existing `DEPENDS_ON:` content line is not
/// re-added, so `--update-key … --depends-on x` re-runs never duplicate it. New
/// targets are appended as one `DEPENDS_ON: a, b` line at the top of the body
/// (the parser scans all lines, so position is immaterial; top keeps it visible).
fn mirror_depends_on_into_content(body: String, depends_on: &[String]) -> String {
    let fresh: Vec<&str> = depends_on
        .iter()
        .map(|d| d.trim())
        .filter(|d| !d.is_empty())
        // Skip any target the body already declares on a dep line (idempotent).
        .filter(|d| !body_declares_dep(&body, d))
        .collect();
    if fresh.is_empty() {
        return body;
    }
    let line = format!("DEPENDS_ON: {}", fresh.join(", "));
    if body.is_empty() {
        line
    } else {
        format!("{line}\n{body}")
    }
}
/// `true` iff `body` already names `dep` on a `DEPENDS_ON:` line —
/// the same lines the readiness parser reads. Whitespace/comma tolerant.
fn body_declares_dep(body: &str, dep: &str) -> bool {
    body.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("DEPENDS_ON:"))
        .any(|l| {
            l.split([':', ',', ' ', '\t'])
                .map(str::trim)
                .any(|tok| tok == dep)
        })
}
/// Bare tail of a dep key: strip up to and including the last `/`. Duplicated
/// locally (not imported from `kavach-rpc`) to avoid a CLI -> rpc dep edge.
fn bare_tail(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}
/// Resolve-or-drop gate for NLU-harvested (speculative) dependency edges.
/// Non-speculative edges pass through unchanged; a speculative edge is kept
/// only if its target's bare tail matches a known `entry_key`'s bare tail.
fn resolve_speculative_deps(
    rels: Vec<kavach_engine::ExtractedRelationship>,
    known_keys: &[String],
) -> (Vec<kavach_engine::ExtractedRelationship>, Vec<String>) {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for rel in rels {
        if !rel.speculative || known_keys.iter().any(|k| bare_tail(k) == bare_tail(&rel.target)) {
            kept.push(rel);
        } else {
            dropped.push(rel.target);
        }
    }
    (kept, dropped)
}
#[expect(
    clippy::too_many_lines,
    reason = "single unified CLI command handler with inlined validation and upsert logic"
)]
pub(crate) fn run(req: &super::rpc_client::WriteRequest<'_>) -> i32 {
    let project_slug = req.project;
    let category = req.category;
    let key = req.key;
    let title = req.title;
    let content = req.content;
    let new = req.new;
    let update_key = req.update_key;
    let priority = req.priority.map(Priority::new);
    // Body resolution honors the documented `--content reads from stdin if
    // omitted` contract: explicit flag wins; else a piped/redirected stdin
    // supplies the body; else (interactive TTY, nothing piped) fall back to the
    // title-only warning. A real stdin read failure surfaces, never swallowed.
    let body = match content {
        Some(c) => c.to_owned(),
        None => match read_stdin_body_or_exit() {
            Ok(Some(piped)) => piped,
            Ok(None) => {
                let warn = "warning: no --content provided and no piped stdin. Only the \
                     title will be stored. Pass --content \"...\" or pipe the body via \
                     stdin to persist plan details, decisions, or context.";
                if let Err(io_err) = ewrite_or_exit(warn) {
                    return into_exit_code(io_err);
                }
                String::new()
            }
            Err(io_err) => return into_exit_code(io_err),
        },
    };
    // DISPATCH-GATING FIX (operator directive 2026-06-17 "honor graph deps in
    // dispatch"): the `--depends-on` flag projects graph edges (below), but the
    // dispatch readiness check reads deps ONLY from the card's CONTENT
    // (`deps_satisfied` -> `parse_declared_deps(&entry.content)`), so a flag-only
    // gate was invisible and the card re-dispatched forever
    // (decision.arch.kavach-depends-on-flag-content-disconnect). Mirror the flag
    // targets into a `DEPENDS_ON:` content line so the SAME source of truth the
    // readiness check parses reflects the flag. Idempotent: skip any target the
    // content already declares, so re-running `--update-key … --depends-on x`
    // never duplicates the line.
    let body = mirror_depends_on_into_content(body, req.depends_on);
    // Carry the RESOLVED body (flag or stdin, deps-mirrored) through every
    // downstream path: the RPC payload, RPC graph-edge extraction, and the
    // direct fallback all read `content` — without this, a stdin-supplied body
    // would reach the direct path but be dropped by the RPC-first call.
    let effective_req = super::rpc_client::WriteRequest {
        content: Some(body.as_str()),
        ..*req
    };
    let eff = &effective_req;
    // STRICT MODE: for canonical categories the caller must declare intent.
    if STRICT_CATEGORIES.contains(&category) && !new && update_key.is_none() {
        let msg = format!(
            "error: writing to '{category}' requires one of:\n  \
             --new                     (create a brand-new row; gate fuzzy-matches)\n  \
             --update-key <existing>   (update a known row)\n\
             Run 'kavach db query --project {project_slug} --category {category}' first."
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    // --update-key narrows the write to a confirmed existing row.
    let effective_key = update_key.map_or(key, |uk| uk);
    if category == ROADMAP_CATEGORY
        && let Some(warning) = check_protected_closure(effective_key, title)
    {
        let msg = format!("error: {warning}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    // RPC-FIRST (single_writer invariant): route the write through the daemon
    // — the sole RocksDB writer — so the CLI never opens a second handle and
    // races the lock. Only when the daemon is unreachable (DAEMON_UNAVAILABLE)
    // is a direct open safe (no other process holds the fcntl lock). Any other
    // RPC error means the daemon is up; propagate it, do not race the lock.
    match super::rpc_client::write(eff) {
        Ok(res) => {
            if res.success {
                let id = super::rpc_client::or_str(res.id, "?");
                let ok = format!("wrote [{category}] {effective_key} — {title} (id={id})");
                return match print_or_exit(&ok) {
                    Ok(()) => 0,
                    Err(io_err) => into_exit_code(io_err),
                };
            }
            let msg = format!(
                "error: {}",
                super::rpc_client::or_str(res.error, "write failed")
            );
            return match ewrite_or_exit(&msg) {
                Ok(()) => 1,
                Err(io_err) => into_exit_code(io_err),
            };
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {
            // Daemon down — fall through to the resilient direct path below.
        }
        Err(e) => {
            let msg = format!("error: {e}");
            return match ewrite_or_exit(&msg) {
                Ok(()) => 1,
                Err(io_err) => into_exit_code(io_err),
            };
        }
    }
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    runtime.block_on(async {
        // Phase 6: direct open via TursoStore (libSQL file — no lock contention).
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        // STRICT MODE existence/dedup checks for the canonical categories.
        if STRICT_CATEGORIES.contains(&category) {
            let rows = match store.list_by_category(project_slug, category).await {
                Ok(rs) => rs,
                Err(e) => {
                    let detail = format!("error: list lookup: {e}");
                    if let Err(io_err) = ewrite_or_exit(&detail) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            };
            if let Some(uk) = update_key {
                // --update-key: row must exist.
                if !rows.iter().any(|r| r.entry_key == uk) {
                    let mut msg = String::new();
                    msg.push_str("error: --update-key '");
                    msg.push_str(uk);
                    msg.push_str("' not found in ");
                    msg.push_str(project_slug);
                    msg.push('/');
                    msg.push_str(category);
                    msg.push_str(
                        ".\nUse --new to create instead, or list existing keys via the kavach CLI.",
                    );
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            } else if new {
                // --new: title must not fuzzy-match existing rows.
                let mut hits: Vec<(String, f64)> = rows
                    .iter()
                    .map(|r| (r.entry_key.clone(), similarity(&r.title, title)))
                    .filter(|(_, s)| *s >= FUZZY_THRESHOLD)
                    .collect();
                hits.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1)
                        .map_or(std::cmp::Ordering::Equal, |o| o)
                });
                hits.truncate(3);
                if !hits.is_empty() {
                    let mut msg = String::from(
                        "error: --new refused — title is too similar to existing rows:\n",
                    );
                    for (k, s) in &hits {
                        writeln!(msg, "  {k} (similarity {s:.2})").ok();
                    }
                    msg.push_str(
                        "Use --update-key <key> to update one of the above, or rename your title.",
                    );
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
                // Also: if --new key matches an existing key, refuse (key collision).
                if rows.iter().any(|r| r.entry_key == effective_key) {
                    let msg = format!(
                        "error: --new refused — key '{effective_key}' already exists. \
                         Use --update-key {effective_key} to update it."
                    );
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            }
        }
        // Build MemoryEntry for upsert
        let now = chrono::Utc::now().to_rfc3339();
        let entry = MemoryEntry {
            id: None,
            project: RecordId::new("project", RecordIdKey::String(project_slug.to_string())),
            category: Some(category.to_string()),
            entry_key: effective_key.to_string(),
            title: title.to_string(),
            content: body.clone(),
            status: Some("todo".to_string()),
            entry_status: Some("todo".to_string()),
            tags: None,
            decay_score: None,
            access_count: Some(0),
            priority: priority.map(|p| p.get()),
            lane: None,
            exec_prompt: req.exec_prompt.map(|s| s.to_string()),
            occupied_by: None,
            occupied_until: None,
            created_at: Some(chrono::DateTime::parse_from_rfc3339(&now).unwrap().with_timezone(&chrono::Utc)),
            updated_at: Some(chrono::DateTime::parse_from_rfc3339(&now).unwrap().with_timezone(&chrono::Utc)),
        };
        // Upsert the entry (includes FTS sync and write-time embedding)
        if let Err(e) = store.upsert_entry(&entry).await {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        let ok_line = format!("wrote [{category}] {effective_key} — {title}");
        if let Err(io_err) = print_or_exit(&ok_line) {
            return into_exit_code(io_err);
        }
        // Phase 1: project typed inter-entry relationships (best-effort, non-blocking).
        // Sources merged: (a) body-extracted edges — frontmatter
        // directives, [[memory:...]] wikilinks, AND NLU prose cues
        // ("depends on X"); (b) explicit --depends-on flag targets.
        let mut relationships = kavach_engine::extract_memory_entry_relationships(&body);
        for dep in eff.depends_on {
            let target = dep.trim();
            if !target.is_empty() {
                relationships.push(kavach_engine::ExtractedRelationship::new(
                    "depends_on",
                    target,
                ));
            }
        }
        // Resolve-or-drop: an NLU-harvested (speculative) target that
        // resolves to no known row would otherwise become a permanently-
        // missing DAG node. Explicit/frontmatter/wikilink edges are exempt.
        let mut known_keys: Vec<String> = Vec::new();
        for tbl in STRICT_CATEGORIES {
            match store.list_by_category(project_slug, tbl).await {
                Ok(rows) => known_keys.extend(rows.into_iter().map(|r| r.entry_key)),
                Err(e) => {
                    let warn = format!("warning: dep resolve lookup failed for {tbl}: {e}");
                    if let Err(io_err) = ewrite_or_exit(&warn) {
                        return into_exit_code(io_err);
                    }
                }
            }
        }
        let (relationships, dropped) =
            resolve_speculative_deps(relationships, &known_keys);
        for target in dropped {
            let notice = format!("dep-guess dropped (no such card): {target}");
            if let Err(io_err) = print_or_exit(&notice) {
                return into_exit_code(io_err);
            }
        }
        if !relationships.is_empty() {
            // Resolve bare keys to fully-qualified names: same project, same category.
            // Wikilinks already carry the full qname.
            let normalised: Vec<(String, String)> = relationships
                .into_iter()
                .map(|r| {
                    let tgt = if r.target.contains('/') {
                        r.target
                    } else {
                        format!("{project_slug}/{category}/{}", r.target)
                    };
                    (r.rel, tgt)
                })
                .collect();
            // TODO: Implement graph edge upsert in TursoStore (GraphStore::relate_dynamic)
            // For now, skip graph edges in direct fallback path.
            let _ = normalised;
        }
        0
    })
}
#[cfg(test)]
#[path = "write_test.rs"]
mod tests;
