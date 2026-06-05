use crate::error::internal;
use jsonrpsee::types::ErrorObjectOwned;

pub(super) const ROADMAP_TABLE: &str = "roadmap";

/// Return option value or default without `unwrap_or` (avoids `clippy::manual_unwrap_or`).
/// Per <https://rust-lang.github.io/rust-clippy/master/index.html#manual_unwrap_or>
pub(super) const fn or_str<'a>(opt: Option<&'a str>, default: &'a str) -> &'a str {
    if let Some(t) = opt {
        return t;
    }
    default
}

pub(super) async fn resolve_project_id(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    slug: &str,
) -> Result<surrealdb_types::RecordId, ErrorObjectOwned> {
    let project = kavach_surreal::project_get_by_slug(db, slug)
        .await
        .map_err(|e| internal(e.to_string()))?;
    match project {
        Some(p) => {
            p.id.map_or_else(|| Err(internal(format!("project has no id: {slug}"))), Ok)
        }
        None => Err(internal(format!("project not found: {slug}"))),
    }
}
