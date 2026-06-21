use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{err_exit, into_exit_code, print_or_exit};

/// Purge an `anti_pattern` cluster by gate. `--confirm` is required: a purge is
/// destructive (drops recurrence history), so an unconfirmed call is a no-op that
/// names what would be removed.
pub(super) fn run(gate: &str, confirm: bool) -> i32 {
    if !confirm {
        return err_exit(&format!(
            "refusing to purge gate '{gate}' without --confirm (destructive: drops the \
             anti_pattern cluster + its mistake_event history)"
        ));
    }
    match rpc_client::mistake_purge(gate) {
        Ok(r) => {
            let line = format!("purged gate '{gate}': {} anti_pattern cluster(s) removed", r.removed);
            match print_or_exit(&line) {
                Ok(()) => 0,
                Err(io) => into_exit_code(io),
            }
        }
        Err(e) => err_exit(&format!("purge: {e}")),
    }
}
