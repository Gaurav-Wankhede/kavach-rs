//! Detect orphaned code: new `.rs` files missing `mod` declarations, and new
//! `pub`/`export` items that need wiring. `rust` covers `.rs`; `js` covers
//! `.tsx`/`.jsx`/`.ts`/`.js`.
mod js;
mod rust;

#[cfg(test)]
mod tests;

use std::path::Path;

/// Returns an orphan-risk advisory string, or None when clean.
pub(crate) fn check_orphan_risk(file_path: &str, content: &str) -> Option<String> {
    if file_path.is_empty() || content.is_empty() {
        return None;
    }
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "rs" => rust::check_rust_orphan(file_path, content),
        "tsx" | "jsx" | "ts" | "js" => js::check_js_orphan(file_path, content),
        _ => None,
    }
}
