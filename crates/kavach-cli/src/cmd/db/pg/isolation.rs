// ARCH: see kavach db get --category decision --key arch.decision.silent_io_guard_shipped
// ALGO: HashSet membership for FK-table classification (preserved verbatim; not modified by this silent-IO migration). TIME: O(F+T) for F FKs and T tables. SOURCE: https://doc.rust-lang.org/std/collections/struct.HashSet.html
use std::collections::HashSet;

use super::schema::{connect, list_foreign_keys, list_tables};
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn run(dsn: &str) -> i32 {
    let mut client = match connect(dsn) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("connect error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let tables = match list_tables(&mut client) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("list tables failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let fks = match list_foreign_keys(&mut client) {
        Ok(f) => f,
        Err(e) => {
            let msg = format!("list fks failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let mut referenced: HashSet<String> = HashSet::new();
    let mut referencing: HashSet<String> = HashSet::new();
    for fk in &fks {
        referenced.insert(format!("{}.{}", fk.parent_schema, fk.parent_table));
        referencing.insert(format!("{}.{}", fk.child_schema, fk.child_table));
    }
    if let Err(io_err) = print_or_exit("# Isolated tables — no FK in OR out\n") {
        return into_exit_code(io_err);
    }
    let mut isolated_count = 0usize;
    let mut root_count = 0usize;
    for t in &tables {
        let key = format!("{}.{}", t.schema, t.name);
        let is_ref = referenced.contains(&key);
        let is_ing = referencing.contains(&key);
        if !is_ref && !is_ing {
            let line = format!("[ISOLATED] {key}");
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
            isolated_count = isolated_count.saturating_add(1);
        } else if is_ref && !is_ing {
            let line = format!("[ROOT]     {key} (referenced by children, no parents)");
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
            root_count = root_count.saturating_add(1);
        }
    }
    let summary = format!(
        "\nsummary: {isolated_count} isolated, {root_count} root, {} total tables",
        tables.len()
    );
    if let Err(io_err) = print_or_exit(&summary) {
        return into_exit_code(io_err);
    }
    0
}
