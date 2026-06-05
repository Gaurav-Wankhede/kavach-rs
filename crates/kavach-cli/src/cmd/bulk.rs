// kavach bulk — sweep-mode CLI hub (dispatches to start | status | close).
// SOURCE: roadmap.unit.kavach-bulk-mode acceptance #6.
mod close;
mod start;
mod status;
mod types;

pub(crate) use types::{BulkAction, BulkArgs};

pub(crate) fn run(args: BulkArgs) -> i32 {
    match args.action {
        BulkAction::Start {
            sweep_id,
            project,
            rca_file,
            scope_glob,
            lint_class,
            fix_strategy,
            blast_estimate,
            ttl_seconds,
            approved_by,
        } => start::run(start::StartParams {
            sweep_id: &sweep_id,
            project: &project,
            rca_file: &rca_file,
            scope_glob: &scope_glob,
            lint_class: &lint_class,
            fix_strategy: &fix_strategy,
            blast_estimate,
            ttl_seconds,
            approved_by: &approved_by,
        }),
        BulkAction::Status { project } => status::run(&project),
        BulkAction::Close { sweep_id, reason } => close::run(&sweep_id, &reason),
    }
}
