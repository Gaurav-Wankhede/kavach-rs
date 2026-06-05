use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn split_csv(s: Option<&str>) -> Vec<String> {
    s.map_or_else(Vec::new, |raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect()
    })
}

pub(super) fn emit_concept_rows(rows: &[kavach_surreal::Entity]) -> i32 {
    if rows.is_empty() {
        return match print_or_exit("no concepts found") {
            Ok(()) => 0,
            Err(io) => into_exit_code(io),
        };
    }
    for row in rows {
        let line = format!("  - {}", row.name);
        match print_or_exit(&line) {
            Ok(()) => {}
            Err(io) => return into_exit_code(io),
        }
    }
    0
}

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
