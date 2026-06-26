// hub: intentional `kavach db` subcommand dispatch hub — mod declarations +
// the single shared `validate_project_workdir` validator called by every db
// subcommand (the sanctioned dispatch-hub helper, cf. cmd/mod.rs dispatch()).
// This change adds only `pub(crate) mod write;` so `cli::db` can reference
// `write::CATEGORY_HELP` — the category single-source-of-truth
// (rca.kavach-db-write-category-enum-inconsistent).
mod archive;
mod backfill_relationships;
mod bridge;
mod citation;
mod concept;
mod delete;
mod delete_prefix;
mod dispatcher;
mod event;
mod exec_prompt_advice;
mod expire;
mod find;
mod flow;
mod gate_config;
mod get;
mod graph_query;
mod infer_deps;
mod kanban;
mod lane;
mod list;
mod mistake_hits;
mod mistake_purge;
mod next_prompt;
mod ope;
mod pg;
mod populate_graph;
mod priority;
mod query;
mod query_raw;
mod register;
mod register_part;
mod rotate;
mod rpc_client;
mod run_rec;
mod search;
mod status_update;
mod sync;
mod tree;
mod wipe_project;
pub(crate) mod write;

pub(crate) use dispatcher::run;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

/// Upsert a roadmap card through the canonical write path (RPC-first → daemon,
/// the single `RocksDB` writer; direct open only as daemon-down fallback). Sends
/// `new: true`, which the daemon's `db.write` treats as a true upsert on
/// `(project, roadmap, key)` — so re-capture of the same key UPDATES one card.
/// Callers outside `db` (e.g. `heal`) use this instead of opening the DB
/// directly, preserving the single-writer invariant. Returns the CLI exit code.
pub(crate) fn upsert_roadmap_card(project: &str, key: &str, title: &str, content: &str) -> i32 {
    write::run(&rpc_client::WriteRequest {
        project,
        category: "roadmap",
        key,
        title,
        content: Some(content),
        new: true,
        update_key: None,
        exec_prompt: None,
        priority: None,
        depends_on: &[],
    })
}

pub(crate) fn validate_project_workdir(project: &kavach_surreal::Project) -> Result<(), i32> {
    match &project.workdir {
        Some(workdir) if std::path::Path::new(workdir).is_dir() => Ok(()),
        Some(workdir) => {
            let slug = &project.slug;
            let msg = format!(
                "error: project '{slug}' workdir does not exist: {workdir}\n\
                 Virtual projects are not allowed. Either:\n  \
                 1. Create the directory: mkdir -p {workdir}\n  \
                 2. Update the project path: kavach db update-project --slug {slug} --path <valid_path>\n  \
                 3. Remove the stale project: kavach db unregister --slug {slug}"
            );
            if let Err(e) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(e));
            }
            Err(1)
        }
        None => {
            let slug = &project.slug;
            let msg = format!(
                "error: project '{slug}' has no workdir configured.\n\
                 Register with: kavach db register --slug {slug} --path <abs_path>"
            );
            if let Err(e) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(e));
            }
            Err(1)
        }
    }
}
