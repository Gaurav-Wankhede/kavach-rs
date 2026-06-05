//! Extract the command that follows a `source .env` / `. .env` invocation.
use super::offset::map_lc_offset_to_command;
use super::scan::downstream_start_in_lc;

/// Extract the command that follows a `source .env` or `. .env` invocation
/// in command position.
///
/// Finds the command-position occurrence of `source ` or `. ` (following a
/// shell separator or start-of-line), advances past the filename, skips any
/// shell redirects (`2>/dev/null`, `>/dev/null 2>&1`), then strips a
/// separator (`&&` or `;`) to return the downstream command.
/// Returns `None` if the match is not in command position (e.g. `--source` flag),
/// or if there is no downstream command.
pub(crate) fn extract_post_source_command(command: &str) -> Option<String> {
    let lc = command.to_lowercase();
    for needle in ["source ", ". "] {
        if let Some(start_lc) = downstream_start_in_lc(&lc, needle) {
            // Map the `lc`-relative downstream byte offset back to `command` so
            // the returned slice preserves original case (e.g. `$DATABASE_URL`).
            let start_cmd = map_lc_offset_to_command(command, &lc, start_lc)?;
            return Some(command.get(start_cmd..)?.to_owned());
        }
    }
    None
}
