/// Bare inner string of a record-id key — the composite `project:entry_key`
/// form used across the unified `memories` table.
#[must_use]
pub fn project_key_str(id: &String) -> String {
    id.clone()
}
