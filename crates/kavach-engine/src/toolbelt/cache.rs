//! Process-wide binary-availability cache for Tool::is_available.
//! See decision.engine.toolbelt-cache-design for architecture.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn availability_cache() -> &'static Mutex<HashMap<String, bool>> {
    static CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Check if a program exists in PATH. Uses the `which` crate for cross-platform
/// (Windows/Linux/macOS) support, with process-wide cache to avoid repeat lookups.
pub(super) fn which(program: &str) -> bool {
    if let Ok(cache) = availability_cache().lock()
        && let Some(&cached) = cache.get(program)
    {
        return cached;
    }
    let found = ::which::which(program).is_ok();
    if let Ok(mut cache) = availability_cache().lock() {
        cache.insert(program.to_owned(), found);
    }
    found
}
