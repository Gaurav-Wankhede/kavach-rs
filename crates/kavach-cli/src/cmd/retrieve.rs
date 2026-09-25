//! Retrieve cached CCR blob by hash or prefix.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};
use std::io::{self, Write};

/// Run the `kavach retrieve <hash>` command.
pub(crate) fn run(hash: &str) -> i32 {
    match kavach_engine::ccr::load_blob(hash) {
        Ok(content) => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            if let Err(e) = handle.write_all(content.as_bytes()) {
                return into_exit_code(e.into());
            }
            0
        }
        Err(e) => {
            let msg = format!("[kavach retrieve] ERROR: {e}");
            match ewrite_or_exit(&msg) {
                Ok(()) => 1,
                Err(io_err) => into_exit_code(io_err),
            }
        }
    }
}
