// ONE-SHOT BACKFILL: derive typed inter-entry edges from row semantics.
//
// [RCA]
// symptom:     Dashboard graph empty; frontmatter extractor yields ~0 edges
//              because legacy content was never authored with annotations.
// why5:        System assumed humans annotate relationships; reality is humans
//              write prose and the system must infer structure.
// root_cause:  Missing data-driven inference layer.
// fix:         Use kavach_engine::infer_relationships to derive edges from
//              titles, content, key prefixes, status, and timestamps.
//
// ALGO: ProjectScopedInfer
// PROBLEM_CLASS: graph_backfill
// TIME: O(p * n^2) where p=projects, n=rows/project | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: Idempotent via UPSERT in upsert_relationships; safe to re-run.
// SOURCE: https://surrealdb.com/docs/surrealql/statements/relate
// SOURCE: https://en.wikipedia.org/wiki/Jaccard_index
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

const CATEGORIES: &[&str] = &["roadmap", "decision", "research", "pattern", "app_spec"];

#[expect(
    clippy::too_many_lines,
    reason = "backfill orchestration: multiple error paths + loop tiers"
)]
pub(super) fn run(project_filter: Option<&str>, dry_run: bool) -> i32 {
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
        let projects = match kavach_surreal::projects_list_all(&db).await {
            Ok(ps) => ps,
            Err(e) => {
                let msg = format!("error: list projects: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        let mut total_rows = 0usize;
        let mut total_edges_planned = 0usize;
        let mut total_edges_written = 0usize;
        for project in &projects {
            if let Some(filter) = project_filter
                && project.slug != filter
            {
                continue;
            }
            let Some(project_id) = project.id.clone() else { continue };

            let mut infer_rows: Vec<kavach_engine::InferRow> = Vec::new();
            for category in CATEGORIES {
                let rows = match kavach_surreal::list_by_project(&db, category, &project_id).await {
                    Ok(rs) => rs,
                    Err(e) => {
                        let msg = format!("warning: list {}/{}: {e}", project.slug, category);
                        if let Err(io_err) = ewrite_or_exit(&msg) {
                            return into_exit_code(io_err);
                        }
                        continue;
                    }
                };
                for row in rows {
                    infer_rows.push(kavach_engine::InferRow {
                        project_slug: project.slug.clone(),
                        category: (*category).to_owned(),
                        entry_key: row.entry_key,
                        content: row.content,
                    });
                }
            }
            total_rows = total_rows.saturating_add(infer_rows.len());
            let edges = kavach_engine::infer_relationships(&infer_rows);
            total_edges_planned = total_edges_planned.saturating_add(edges.len());
            let summary_line = format!(
                "[{}] rows={} inferred_edges={}",
                project.slug,
                infer_rows.len(),
                edges.len()
            );
            if let Err(io_err) = print_or_exit(&summary_line) {
                return into_exit_code(io_err);
            }
            if dry_run {
                let preview = edges.iter().take(10);
                for e in preview {
                    let line = format!(
                        "  [dry-run] {} --{}--> {}",
                        e.source_qname, e.rel, e.target_qname
                    );
                    if let Err(io_err) = print_or_exit(&line) {
                        return into_exit_code(io_err);
                    }
                }
                continue;
            }
            let mut by_source: std::collections::HashMap<String, Vec<(String, String)>> =
                std::collections::HashMap::new();
            for e in edges {
                by_source
                    .entry(e.source_qname)
                    .or_default()
                    .push((e.rel, e.target_qname));
            }
            for (src, rels) in by_source {
                match kavach_surreal::upsert_relationships(&db, &src, &rels).await {
                    Ok(n) => total_edges_written = total_edges_written.saturating_add(n),
                    Err(e) => {
                        let msg = format!("warning: upsert_relationships({src}): {e}");
                        if let Err(io_err) = ewrite_or_exit(&msg) {
                            return into_exit_code(io_err);
                        }
                    }
                }
            }
        }
        let total_line = format!(
            "\nbackfill summary: rows={total_rows} edges_planned={total_edges_planned} edges_written={total_edges_written} dry_run={dry_run}"
        );
        if let Err(io_err) = print_or_exit(&total_line) {
            return into_exit_code(io_err);
        }
        0
    })
}
