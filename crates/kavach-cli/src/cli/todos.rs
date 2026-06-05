use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum TodosAction {
    /// Scan source for `kavach_todo`!() invocations and sync to kanban
    Sync {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Project root path
        #[arg(long, default_value = ".")]
        path: String,
        /// Preview changes without writing
        #[arg(long)]
        dry_run: bool,
    },
}
