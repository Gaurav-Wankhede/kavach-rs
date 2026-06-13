// Cache-dir resolution for the embedding model — kept cwd-independent so the
// launchd RPC daemon (cwd=/) finds the stored model instead of re-fetching it.
// SOURCE: rca.embedder-cwd-relative-cache-dark-on-daemon.
//
// ALGO: none — pure path resolution (env lookup + parent-dir join); no search,
//   no data structure. TIME: O(1). SPACE: O(1). YEAR: 2026.
use std::path::PathBuf;

/// Absolute, cwd-independent storage directory for the embedding model.
///
/// fastembed defaults to the *relative* `.fastembed_cache`, resolved against the
/// process cwd. The RPC daemon runs under launchd with cwd `/`, so that default
/// resolved to `/.fastembed_cache` — a path that never holds the model, so every
/// daemon-side embed failed with `Failed to retrieve onnx/model.onnx` and the
/// entire mistake/concept embedding subsystem went dark. Anchoring the store
/// beside the `SharedAI` database makes it independent of how the process was
/// launched. Honors `FASTEMBED_CACHE_DIR` when set.
pub(super) fn model_cache_dir() -> PathBuf {
    resolve(std::env::var("FASTEMBED_CACHE_DIR").ok())
}

/// Pure resolver split out for testing.
///
/// An absolute `FASTEMBED_CACHE_DIR` override wins verbatim; a relative one is
/// anchored under the base (never cwd-relative); with no override the store sits
/// beside the `SharedAI` database. Every branch is absolute — a cwd-relative path
/// is exactly what took the daemon embedder dark, so the override must not be able
/// to reintroduce it.
fn resolve(env_override: Option<String>) -> PathBuf {
    let db = crate::connection::default_db_path();
    // default_db_path is always an absolute, multi-segment path, so its parent is
    // absolute too; the parentless arm is unreachable but still yields an absolute
    // base rather than the cwd-relative `.fastembed_cache` that caused the outage.
    let base = db.parent().unwrap_or(db.as_path());
    env_override.map_or_else(
        || base.join("fastembed_cache"),
        |dir| {
            let p = PathBuf::from(dir);
            if p.is_absolute() { p } else { base.join(p) }
        },
    )
}

#[cfg(test)]
#[path = "cache_dir_test.rs"]
mod cache_dir_test;
