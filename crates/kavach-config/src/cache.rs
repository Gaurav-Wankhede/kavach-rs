use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const CACHE_TTL_SECS: u64 = 300;
const MAX_CACHE_SIZE: usize = 50;

struct CacheEntry {
    data: HashMap<String, Vec<String>>,
    timestamp: Instant,
    last_access: Instant,
}

static PATTERN_CACHE: std::sync::LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API used cross-module; pub would trip unreachable_pub in this private module"
)]
pub(crate) const TTL: Duration = Duration::from_secs(CACHE_TTL_SECS);

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API used cross-module"
)]
pub(crate) fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("KAVACH_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    // Trusted absolute dirs only — no CWD-relative fallback (CWE-363/CWE-22:
    // a hook in an untrusted repo must not load gate config from it).
    // See decision.config.trusted-config-dirs.
    let home = dirs_home();
    [
        home.join(".config").join("kavach"),
        PathBuf::from("/etc/kavach"),
    ]
    .into_iter()
    .find(|c| c.exists())
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API used cross-module"
)]
pub(crate) fn dirs_home() -> PathBuf {
    std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from)
}

pub fn load_patterns(filename: &str) -> HashMap<String, Vec<String>> {
    let mut cache = PATTERN_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = cache.get_mut(filename)
        && entry.timestamp.elapsed() < TTL
    {
        entry.last_access = Instant::now();
        return entry.data.clone();
    }
    //   traversal surface this CWE-363 fix closes"},{"name":"keep CWD
    //   ./config fallback","reason":"untrusted-dir gate-config tampering"}]
    // TIME: O(1) | SPACE: O(1)
    // YEAR: 2026 | SEARCHED: 2026-05
    // CWE-363 fix: CWD-relative "config/" fallback removed — a hook runs in
    // untrusted project dirs and must not read gate patterns from there.
    let mut paths = Vec::new();
    if let Some(dir) = config_dir() {
        paths.push(dir.join(filename));
    }
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    let content = paths.iter().find_map(|p| fs::read_to_string(p).ok());
    if let Some(text) = content {
        let mut section = String::new();
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if t.starts_with('[') && t.ends_with(']') {
                if let Some(inner) = t.get(1..t.len().saturating_sub(1)) {
                    inner.clone_into(&mut section);
                }
                continue;
            }
            if !section.is_empty() {
                if let Some(rest) = t.strip_prefix("keywords:") {
                    for kw in rest.split(',') {
                        let kw = kw.trim();
                        if !kw.is_empty() {
                            result
                                .entry(section.clone())
                                .or_default()
                                .push(kw.to_owned());
                        }
                    }
                } else {
                    result
                        .entry(section.clone())
                        .or_default()
                        .push(t.to_owned());
                }
            }
        }
    }
    if cache.len() >= MAX_CACHE_SIZE {
        evict_lru(&mut cache);
    }
    let now = Instant::now();
    cache.insert(
        filename.to_owned(),
        CacheEntry {
            data: result.clone(),
            timestamp: now,
            last_access: now,
        },
    );
    result
}

fn evict_lru(cache: &mut HashMap<String, CacheEntry>) {
    let oldest = cache
        .iter()
        .min_by_key(|(_, e)| e.last_access)
        .map(|(k, _)| k.clone());
    if let Some(key) = oldest {
        cache.remove(&key);
    }
}

pub fn clear_cache() {
    PATTERN_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_patterns_empty_file() {
        let result = load_patterns("nonexistent.toon");
        let _ = result;
    }
}
