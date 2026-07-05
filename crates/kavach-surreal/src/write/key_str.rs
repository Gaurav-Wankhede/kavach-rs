use surrealdb_types::{RecordId, RecordIdKey};

/// Bare inner string of a record-id key — never the `String("x")` Debug wrapper that double-nests composite ids. SOURCE: decision.bug.recordid-nested-key-str.
#[must_use]
pub(super) fn project_key_str(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
#[path = "key_str_test.rs"]
mod key_str_test;
