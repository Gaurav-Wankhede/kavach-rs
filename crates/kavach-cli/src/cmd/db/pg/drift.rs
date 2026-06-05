// ARCH: see kavach db get --category decision --key arch.decision.silent_io_guard_shipped
// ALGO: HashMap+HashSet lookup for FK drift detection (preserved verbatim; not modified by this silent-IO migration). TIME: O(T+F+C). SOURCE: https://doc.rust-lang.org/std/collections/struct.HashMap.html
use std::collections::{HashMap, HashSet};

use super::schema::{
    Column, ForeignKey, Table, connect, list_columns, list_foreign_keys, list_tables,
};
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Detect columns named `<something>_id` or `<something>_uuid` that lack a declared FK.
/// If a table named `<something>` exists, the column is a likely missing FK.
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
    let columns = match list_columns(&mut client) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("list columns failed: {e}");
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
    let drift = find_drift(&tables, &columns, &fks);
    if let Err(io_err) = print_or_exit("# Missing FK drift report\n") {
        return into_exit_code(io_err);
    }
    if drift.is_empty() {
        if let Err(io_err) = print_or_exit("no drift detected") {
            return into_exit_code(io_err);
        }
        return 0;
    }
    for entry in &drift {
        let head = format!(
            "{}.{}.{}  →  likely FK to {}.{}",
            entry.child_schema,
            entry.child_table,
            entry.child_column,
            entry.likely_parent_schema,
            entry.likely_parent_table,
        );
        if let Err(io_err) = print_or_exit(&head) {
            return into_exit_code(io_err);
        }
        let fix = format!(
            "    fix: ALTER TABLE {}.{} ADD CONSTRAINT fk_{}_{}  FOREIGN KEY ({}) REFERENCES {}.{}(id);",
            entry.child_schema,
            entry.child_table,
            entry.child_table,
            entry.child_column,
            entry.child_column,
            entry.likely_parent_schema,
            entry.likely_parent_table,
        );
        if let Err(io_err) = print_or_exit(&fix) {
            return into_exit_code(io_err);
        }
    }
    let summary = format!("\n{} drift candidate(s)", drift.len());
    if let Err(io_err) = print_or_exit(&summary) {
        return into_exit_code(io_err);
    }
    0
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DriftEntry {
    child_schema: String,
    child_table: String,
    child_column: String,
    likely_parent_schema: String,
    likely_parent_table: String,
}

fn find_drift(tables: &[Table], columns: &[Column], fks: &[ForeignKey]) -> Vec<DriftEntry> {
    // Table names for quick lookup: name → (schema, name).
    let mut table_by_name: HashMap<String, (String, String)> = HashMap::with_capacity(tables.len());
    for t in tables {
        table_by_name.insert(t.name.clone(), (t.schema.clone(), t.name.clone()));
    }
    // Declared FK columns: (schema, table, column) set for fast "already has FK" lookup.
    let mut declared: HashSet<(String, String, String)> = HashSet::with_capacity(fks.len());
    for fk in fks {
        declared.insert((
            fk.child_schema.clone(),
            fk.child_table.clone(),
            fk.child_column.clone(),
        ));
    }
    let mut out: Vec<DriftEntry> = Vec::new();
    for col in columns {
        let Some(stem) = name_stem(&col.name) else {
            continue;
        };
        // Column named `<stem>_id` or `<stem>_uuid`. Skip if the column already has a declared FK.
        let key = (col.schema.clone(), col.table.clone(), col.name.clone());
        if declared.contains(&key) {
            continue;
        }
        // Skip self-references (e.g. `parent_id` on a tree table where col stem == table name).
        if stem == col.table {
            continue;
        }
        // Try both exact match and plural → singular (drop trailing 's').
        let parent = table_by_name.get(stem.as_str()).or_else(|| {
            let plural = format!("{stem}s");
            table_by_name.get(plural.as_str())
        });
        if let Some((parent_schema, parent_table)) = parent {
            out.push(DriftEntry {
                child_schema: col.schema.clone(),
                child_table: col.table.clone(),
                child_column: col.name.clone(),
                likely_parent_schema: parent_schema.clone(),
                likely_parent_table: parent_table.clone(),
            });
        }
    }
    out
}

/// Extract the `<stem>` from `<stem>_id` / `<stem>_uuid`. Returns None if column name
/// does not end with `_id` / `_uuid`.
fn name_stem(col: &str) -> Option<String> {
    if let Some(stripped) = col.strip_suffix("_id")
        && !stripped.is_empty()
    {
        return Some(stripped.to_owned());
    }
    if let Some(stripped) = col.strip_suffix("_uuid")
        && !stripped.is_empty()
    {
        return Some(stripped.to_owned());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_strip_id_suffix() {
        assert_eq!(name_stem("user_id"), Some("user".to_owned()));
        assert_eq!(name_stem("post_uuid"), Some("post".to_owned()));
    }

    #[test]
    fn should_return_none_for_non_fk_columns() {
        assert_eq!(name_stem("created_at"), None);
        assert_eq!(name_stem("email"), None);
        assert_eq!(name_stem("id"), None); // bare "id" is the PK, not an FK
    }

    #[test]
    fn should_detect_missing_fk_when_parent_table_exists() {
        let tables = vec![
            Table {
                schema: "public".into(),
                name: "users".into(),
            },
            Table {
                schema: "public".into(),
                name: "posts".into(),
            },
        ];
        let columns = vec![Column {
            schema: "public".into(),
            table: "posts".into(),
            name: "user_id".into(),
        }];
        let fks: Vec<ForeignKey> = vec![]; // no FKs declared
        let drift = find_drift(&tables, &columns, &fks);
        assert_eq!(drift.len(), 1);
        let entry = drift.first().expect("should have one entry");
        assert_eq!(entry.child_table, "posts");
        assert_eq!(entry.child_column, "user_id");
        assert_eq!(entry.likely_parent_table, "users");
    }

    #[test]
    fn should_skip_column_when_fk_already_declared() {
        let tables = vec![
            Table {
                schema: "public".into(),
                name: "users".into(),
            },
            Table {
                schema: "public".into(),
                name: "posts".into(),
            },
        ];
        let columns = vec![Column {
            schema: "public".into(),
            table: "posts".into(),
            name: "user_id".into(),
        }];
        let fks = vec![ForeignKey {
            child_schema: "public".into(),
            child_table: "posts".into(),
            child_column: "user_id".into(),
            parent_schema: "public".into(),
            parent_table: "users".into(),
            parent_column: "id".into(),
            constraint_name: "fk".into(),
        }];
        let drift = find_drift(&tables, &columns, &fks);
        assert!(drift.is_empty());
    }
}
