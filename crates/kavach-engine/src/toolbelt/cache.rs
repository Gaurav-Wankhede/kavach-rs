//! Process-wide binary-availability cache backing `Tool::is_available`.
// ARCH: ProcessWideAvailabilityCache
// PROBLEM_CLASS: cache
// SOURCE: 2026 Rust idiom — OnceLock<Mutex<HashMap>> for low-contention process caches
// CAPACITY: ≤15 unique tools, ≤50 lookups per session
// LATENCY: O(1) cache hit, O(PATH) cache miss (which crate)
// CONTENTION: low — short critical section, gates run sequentially per process
// FAILURE_MODE: cache poisoning on panic → next call re-checks via which::which()
// CONSISTENCY: assumes PATH stable for process lifetime
// REJECTED: [{"name":"RwLock","reason":"contention too low to justify"},{"name":"DashMap","reason":"new dep for ≤15 entries"},{"name":"per-call subprocess","reason":"50× latency"}]
// BENCHMARK: https://oneuptime.com/blog/post/2026-01-25-global-mutable-singletons-rust/view
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
