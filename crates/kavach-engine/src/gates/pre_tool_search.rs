//! Pre-tool search gate: validate `WebSearch` queries against stale training
//! knowledge — block/advise when a query names an older year (`year`) or an
//! older major version than installed (`version`, reading manifests via `deps`).
mod deps;
mod version;
mod year;
#[cfg(test)]
#[path = "pre_tool_search_test.rs"]
mod tests;
use kavach_types::HookInput;
use version::check_stale_version_in_query;
use year::check_stale_year_in_query;
/// Pre-tool search gate: advise when a `WebSearch` query steers toward stale
/// (training-weight) years or versions. Advisory-only — never hard-blocks.
pub(crate) fn run(input: &HookInput) {
    let query = input.get_string("query");
    if query.is_empty() {
        drop(kavach_hook::exit_silent());
        return;
    }
    let current_year = kavach_hook::current_year();
    if let Some(reason) = check_stale_year_in_query(query, current_year) {
        drop(kavach_hook::exit_pre_tool_allow(Some(&format!(
            "[ADVISORY:stale-year-query] {reason}"
        ))));
        return;
    }
    // Stale framework versions: training weights vs actual package.json/Cargo.toml.
    let session = kavach_session::get_or_create_session();
    if let Some(reason) = check_stale_version_in_query(query, &session.work_dir) {
        drop(kavach_hook::exit_pre_tool_allow(Some(&format!(
            "[ADVISORY:stale-version-query] {reason}"
        ))));
        return;
    }
    drop(kavach_hook::exit_silent());
}
