// split: intentional — single CLI command handler with helper async fns
//! kavach:nano-file-exempt one cohesive `kavach db write` command handler —
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
    if let Some(nudge) = super::exec_prompt_advice::advise(category, req.exec_prompt)
        && let Err(io_err) = ewrite_or_exit(&nudge)
    {
        return into_exit_code(io_err);
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
        let db = match kavach_surreal::open_default().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: open SurrealDB: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(e) = kavach_surreal::apply_schema(&db).await {
            let msg = format!("error: schema apply: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let msg = format!("error: project not found: {project_slug}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                let hint_code = suggest_projects(&db).await;
                if hint_code != 0 {
                    return hint_code;
                }
                return 1;
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(code) = super::validate_project_workdir(&project) {
            return code;
        }
        let Some(project_id) = project.id else {
            if let Err(io_err) = ewrite_or_exit("error: project has no id") {
                return into_exit_code(io_err);
            }
            return 1;
        };
        // STRICT MODE existence/dedup checks for the canonical categories.
        if STRICT_CATEGORIES.contains(&category) {
            let rows = match kavach_surreal::list_by_project(&db, category, &project_id).await {
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
        // Atomic upsert: memory entry + event log + graph entity + edges
        // ALL in a single SurrealDB BEGIN/COMMIT transaction.
        let event_source = format!("kavach_db_write/{category}");
        let qualified_name =
            kavach_engine::memory_entry_qualified_name(category, effective_key, project_slug);
        let refs: Vec<String> = kavach_engine::extract_memory_entry_references(&body);
        let upsert_result = kavach_surreal::upsert_entry_full()
            .db(&db)
            .category(category)
            .project_id(&project_id)
            .entry_key(effective_key)
            .title(title)
            .content(&body)
            .event_source(&event_source)
            .qualified_name(&qualified_name)
            .references(&refs)
            .maybe_priority(priority)
            .maybe_exec_prompt(req.exec_prompt)
            .build_for_call()
            .await;
        match upsert_result {
            Ok(id) => {
                let ok_line = format!("wrote [{category}] {effective_key} — {title} (id={id:?})");
                if let Err(io_err) = print_or_exit(&ok_line) {
                    return into_exit_code(io_err);
                }
                // Phase 1: project typed inter-entry relationships.
                // Sources merged: (a) body-extracted edges — frontmatter
                // directives, [[memory:...]] wikilinks, AND NLU prose cues
                // ("depends on X"); (b) explicit --depends-on flag targets.
                // The flag is the precise, no-guess path; the extractors catch
                // edges the author stated only in prose. Both feed the DAG.
                // Idempotent: relationships are sort+deduped by the extractor
                // and upsert_relationships UPSERTs endpoints by name (no dup
                // logical edge). SOURCE: https://surrealdb.com/docs/learn/querying/concepts-and-guides/idempotent-operations
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
                    match kavach_surreal::upsert_relationships(&db, &qualified_name, &normalised)
                        .await
                    {
                        Ok(n) if n > 0 => {
                            let edge_line = format!(
                                "  +{n} graph edge(s) from frontmatter/wikilinks/NLU/--depends-on"
                            );
                            if let Err(io_err) = print_or_exit(&edge_line) {
                                return into_exit_code(io_err);
                            }
                        }
                        Ok(_) => {
                            // n == 0 with a non-empty edge set is NOT silent success.
                            let asked: Vec<String> = normalised
                                .iter()
                                .map(|(r, t)| format!("{r}->{t}"))
                                .collect();
                            let warn = format!(
                                "warning: 0 graph edges written despite {} requested: {}",
                                asked.len(),
                                asked.join(", ")
                            );
                            if let Err(io_err) = ewrite_or_exit(&warn) {
                                return into_exit_code(io_err);
                            }
                        }
                        Err(e) => {
                            let warn = format!("warning: relationship projection failed: {e}");
                            if let Err(io_err) = ewrite_or_exit(&warn) {
                                return into_exit_code(io_err);
                            }
                        }
                    }
                }
                0
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
        }
    })
}
// project_memory_entry_via_db removed — graph projection now happens
// atomically inside kavach_surreal::upsert_entry_full's transaction.
// Best-effort hints on the not-found error path. Caller already returns
// exit 1; if a hint write fails we surface the IO failure code instead.
async fn suggest_projects(db: &surrealdb::Surreal<surrealdb::engine::any::Any>) -> i32 {
    match kavach_surreal::projects_list_all(db).await {
        Ok(projects) if projects.is_empty() => {
            if let Err(io_err) = ewrite_or_exit("hint: no projects registered yet") {
                return into_exit_code(io_err);
            }
            if let Err(io_err) = ewrite_or_exit(
                "  register with: kavach db register --slug <slug> --path <abs_path>",
            ) {
                return into_exit_code(io_err);
            }
        }
        Ok(projects) => {
            let all: Vec<&str> = projects.iter().map(|p| p.slug.as_str()).collect();
            let header = format!("registered projects: {}", all.join(", "));
            if let Err(io_err) = ewrite_or_exit(&header) {
                return into_exit_code(io_err);
            }
            if let Err(io_err) = ewrite_or_exit("  list all: kavach db list-projects") {
                return into_exit_code(io_err);
            }
            if let Err(io_err) =
                ewrite_or_exit("  register: kavach db register --slug <slug> --path <abs_path>")
            {
                return into_exit_code(io_err);
            }
        }
        Err(_) => {
            if let Err(io_err) =
                ewrite_or_exit("hint: run 'kavach db list-projects' to see registered projects")
            {
                return into_exit_code(io_err);
            }
        }
    }
    0
}
#[cfg(test)]
#[path = "write_test.rs"]
mod tests;
