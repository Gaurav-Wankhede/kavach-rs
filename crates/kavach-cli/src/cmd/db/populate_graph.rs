use crate::cmd::io_safe::{into_exit_code, print_or_exit};

/// Deprecation stub. The original SQLite-backed populator was a one-shot
/// bootstrap; data has been migrated to `SurrealDB` and entity/edge writes now
/// happen inline at write time. This command is retained as a no-op so any
/// scripts referencing it keep exiting 0.
pub(super) fn run() -> i32 {
    let msg = "populate-graph: deprecated — graph entities are now written inline against SurrealDB; this is a no-op.";
    if let Err(e) = print_or_exit(msg) {
        return into_exit_code(e);
    }
    0
}
