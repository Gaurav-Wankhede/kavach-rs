//! `.env*` filename detection for accurate error messages — most-specific
//! known variant first, then any `.env.<suffix>`, else the `.env` fallback.

/// Detect which `.env*` filename appears in a command for accurate error messages.
///
/// Scans for `.env.local`, `.env.development`, `.env.production`, `.env.test`,
/// `.env.staging`, `.envrc`, then any `.env.<suffix>` variant. Returns the most
/// specific match found, or `.env` as fallback.
pub(crate) fn detect_env_filename(lc: &str) -> String {
    let variants = [
        ".env.local",
        ".env.development",
        ".env.production",
        ".env.test",
        ".env.staging",
        ".envrc",
    ];
    for v in variants {
        if lc.contains(v) {
            return v.to_owned();
        }
    }
    let Some(idx) = lc.find(".env.") else {
        return ".env".to_owned();
    };
    let Some(after) = lc.get(idx..) else {
        return ".env".to_owned();
    };
    let end = after
        .find(|c: char| c.is_whitespace() || c == ';' || c == '&' || c == '|')
        .unwrap_or(after.len());
    after
        .get(..end)
        .map_or_else(|| ".env".to_owned(), ToOwned::to_owned)
}
