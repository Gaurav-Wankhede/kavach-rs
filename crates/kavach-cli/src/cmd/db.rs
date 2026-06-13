// hub: intentional `kavach db` subcommand dispatch hub — mod declarations +
// the single shared `validate_project_workdir` validator called by every db
// subcommand (the sanctioned dispatch-hub helper, cf. cmd/mod.rs dispatch()).
// This change adds only `pub(crate) mod write;` so `cli::db` can reference
// `write::CATEGORY_HELP` — the category single-source-of-truth
// (rca.kavach-db-write-category-enum-inconsistent).
mod archive;
mod backfill_relationships;
mod bridge;
mod concept;
mod delete;
mod dispatcher;
mod event;
mod expire;
mod find;
mod flow;
mod gate_config;
mod get;
mod graph_query;
mod kanban;
mod lane;
mod list;
mod mistake_hits;
mod pg;
mod populate_graph;
mod priority;
mod query;
mod register;
mod register_part;
mod rotate;
mod rpc_client;
mod search;
mod status_update;
mod sync;
mod tree;
mod wipe_project;
pub(crate) mod write;

pub(crate) use dispatcher::run;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

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
