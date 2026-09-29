use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum WorkingStateAction {
    /// Write a key-value slot (overwrites existing; max 10 slots).
    Set { key: String, value: String },
    /// Read a slot by key. Exit 1 if missing.
    Get { key: String },
    /// Clear all working-state slots.
    Clear,
    /// Print a [STATE] block of all slots to stdout (inject at harness turn start).
    Inject,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::cli::Cli;

    #[test]
    fn working_state_subcommands_are_valid() {
        crate::cli::help_stack::on_big_stack(|| Cli::command().debug_assert());
    }
}
