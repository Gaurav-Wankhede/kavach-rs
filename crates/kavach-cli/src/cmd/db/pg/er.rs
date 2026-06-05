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
    if let Err(io_err) = print_or_exit("```mermaid") {
        return into_exit_code(io_err);
    }
    if let Err(io_err) = print_or_exit("erDiagram") {
        return into_exit_code(io_err);
    }
    for t in &tables {
        let opener = format!("  {} {{", sanitize(&t.name));
        if let Err(io_err) = print_or_exit(&opener) {
            return into_exit_code(io_err);
        }
        if let Err(io_err) = print_or_exit("    int id PK") {
            return into_exit_code(io_err);
        }
        if let Err(io_err) = print_or_exit("  }") {
            return into_exit_code(io_err);
        }
    }
    for fk in &fks {
        let edge = format!(
            "  {} ||--o{{ {} : \"{}\"",
            sanitize(&fk.parent_table),
            sanitize(&fk.child_table),
            fk.child_column,
        );
        if let Err(io_err) = print_or_exit(&edge) {
            return into_exit_code(io_err);
        }
    }
    if let Err(io_err) = print_or_exit("```") {
        return into_exit_code(io_err);
    }
    0
}

/// Mermaid identifiers reject hyphens and quotes — replace them.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
