use crate::error::internal;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_db::turso::TursoStore;

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
    db: &TursoStore,
    slug: &str,
) -> Result<String, ErrorObjectOwned> {
    let project = kavach_db::project_get_by_slug(db, slug)
        .await
        .map_err(|e| internal(e.to_string()))?;
    match project {
        Some(p) => {
            p.id.ok_or_else(|| internal(format!("project has no id: {slug}")))
        }
        None => Err(internal(format!("project not found: {slug}"))),
    }
}
