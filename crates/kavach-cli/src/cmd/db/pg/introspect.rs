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
    if let Err(io_err) = print_or_exit("# PostgreSQL schema introspection\n") {
        return into_exit_code(io_err);
    }
    let tables_header = format!("## Tables ({} total)", tables.len());
    if let Err(io_err) = print_or_exit(&tables_header) {
        return into_exit_code(io_err);
    }
    for t in &tables {
        let line = format!("- {}.{}", t.schema, t.name);
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    let fks_header = format!("\n## Foreign Keys ({} total)", fks.len());
    if let Err(io_err) = print_or_exit(&fks_header) {
        return into_exit_code(io_err);
    }
    for fk in &fks {
        let line = format!(
            "- {}.{}.{} → {}.{}.{} [{}]",
            fk.child_schema,
            fk.child_table,
            fk.child_column,
            fk.parent_schema,
            fk.parent_table,
            fk.parent_column,
            fk.constraint_name,
        );
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    0
}
