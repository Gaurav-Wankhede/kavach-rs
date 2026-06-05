use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(prompt: &str, max_uses: u8) -> i32 {
    match kavach_advisor::ask(prompt, max_uses) {
        Ok(text) => {
            if let Err(e) = print_or_exit(&text) {
                return into_exit_code(e);
            }
            0
        }
        Err(e) => {
            let msg = format!("advisor error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}
