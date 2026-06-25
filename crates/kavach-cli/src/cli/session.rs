use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum SessionAction {
    /// Initialize a new session
    Init,
    /// Validate current session
    Validate,
    /// End the current session
    End,
    /// Compact session state
    Compact,
    /// Resume a previous session
    Resume,
    /// Land session (finalize)
    Land,
    /// End-hook (lifecycle cleanup)
    EndHook,
    /// Clear stale test run locks (use when a test was interrupted and the lock is stuck)
    ClearTestLocks,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::cli::Cli;

    #[test]
    fn session_subcommands_are_valid() {
        crate::cli::help_stack::on_big_stack(|| Cli::command().debug_assert());
    }
}
