//! Auto memory-bank context: project + ancestry roadmap/decision titles.
//! DB lookups (ancestry, titles) live in the `query` submodule.
mod query;

use std::fmt::Write as _;

use query::{query_category_titles, resolve_ancestry};

/// Auto-query kavach-db for project context at session start.
/// Injects titles from child project AND all ancestors (parent inheritance).
/// Injects titles only — no content bodies — saving ~2kB/session.
/// Full content is on-demand: `kavach db get --project <slug> --category <cat> --key <key>`.
pub(super) fn auto_query_memory(project: &str) -> Option<String> {
    // Resolve ancestry chain: child first, then parent, grandparent, ...
    let ancestry = resolve_ancestry(project);

    let mut all_roadmap = String::new();
    let mut all_decisions = String::new();
    let mut ancestor_slugs: Vec<String> = Vec::new();

    for slug in &ancestry {
        if let Some(r) = query_category_titles(slug, "roadmap") {
            all_roadmap.push_str(&r);
            all_roadmap.push('\n');
        }
        if let Some(d) = query_category_titles(slug, "decision") {
            all_decisions.push_str(&d);
            all_decisions.push('\n');
        }
        ancestor_slugs.push(slug.clone());
    }

    // Keep only the last 5 decision titles.
    let decisions_trimmed = all_decisions
        .lines()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    let combined = format!("{all_roadmap}\n{decisions_trimmed}")
        .trim()
        .to_owned();
    if combined.is_empty() {
        return None;
    }

    let mut ctx = String::from("\n[MEMORY_BANK]\n");
    ctx.push_str("status: titles-only (use kavach db get for content)\n");
    writeln!(ctx, "project: {}", ancestor_slugs.join(" → ")).ok();
    ctx.push_str(&combined);
    ctx.push('\n');
    Some(ctx)
}
