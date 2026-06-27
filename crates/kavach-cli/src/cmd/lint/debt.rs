// `kavach lint debt` — harvest simplification-ceiling markers into a debt ledger.
use std::path::Path;
use crate::cmd::io_safe;
use crate::cmd::lint::walk::walk_rs;
const MARKER: &str = "kavach:intentional";
/// One harvested debt row: where, what was simplified, and whether it names an
/// upgrade trigger (a marker with no trigger is the highest rot risk).
struct Debt {
    loc: String,
    note: String,
    has_trigger: bool,
}
fn marker_note(line: &str) -> Option<(String, bool)> {
    let after = line.split(MARKER).nth(1)?.trim();
    // A trigger is named when the note mentions upgrade/when/if/until/once.
    let lower = after.to_lowercase();
    let has_trigger = ["upgrade", "when ", "if ", "until", "once "]
        .iter()
        .any(|t| lower.contains(t));
    Some((after.to_owned(), has_trigger))
}
/// Scan `root` for ceiling markers and print the ledger. Returns 0 always (report-only).
pub(crate) fn run(root: &Path) -> i32 {
    let mut rows: Vec<Debt> = Vec::new();
    walk_rs(root, root, &mut |rel, content| {
        for (i, line) in content.lines().enumerate() {
            if let Some((note, has_trigger)) = marker_note(line) {
                rows.push(Debt {
                    loc: format!("{rel}:{}", i.saturating_add(1)),
                    note,
                    has_trigger,
                });
            }
        }
    });
    emit(&rows)
}
fn emit(rows: &[Debt]) -> i32 {
    if rows.is_empty() {
        return io_safe::print_or_exit("No simplification-ceiling debt. Clean ledger.")
            .map_or_else(io_safe::into_exit_code, |()| 0);
    }
    let no_trigger = rows.iter().filter(|r| !r.has_trigger).count();
    for r in rows {
        let tag = if r.has_trigger { "" } else { " [no-trigger]" };
        let line = format!("  {}{tag}: {}", r.loc, r.note);
        if let Err(e) = io_safe::print_or_exit(&line) {
            return io_safe::into_exit_code(e);
        }
    }
    let summary = format!(
        "{} marker(s), {no_trigger} with no upgrade trigger.",
        rows.len()
    );
    io_safe::print_or_exit(&summary).map_or_else(io_safe::into_exit_code, |()| 0)
}
#[cfg(test)]
#[path = "debt_test.rs"]
#[cfg(test)]
#[path = "debt_test.rs"]
mod tests;
