use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn emit_ok(msg: &str) -> i32 {
    match print_or_exit(msg) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

pub(super) fn emit_err(msg: &str) -> i32 {
    let line = format!("error: {msg}");
    match ewrite_or_exit(&line) {
        Ok(()) => 1,
        Err(io) => into_exit_code(io),
    }
}
