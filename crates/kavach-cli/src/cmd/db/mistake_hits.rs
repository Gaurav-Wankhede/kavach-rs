use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

fn err_exit(msg: &str) -> i32 {
    let line = format!("error: {msg}");
    match ewrite_or_exit(&line) {
        Ok(()) => 1,
        Err(io) => into_exit_code(io),
    }
}

pub(super) fn run(anti_pattern_name: &str) -> i32 {
    match rpc_client::mistake_hit_count(anti_pattern_name) {
        Ok(r) => {
            let line = format!("anti_pattern '{}' hit_count = {}", r.name, r.hit_count);
            match print_or_exit(&line) {
                Ok(()) => 0,
                Err(io) => into_exit_code(io),
            }
        }
        Err(e) => err_exit(&format!("count: {e}")),
    }
}
