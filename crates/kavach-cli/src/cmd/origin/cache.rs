use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::site::{Kind, Site};

const MAX_CACHE_ENTRIES: usize = 50;

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    file: String,
    line: usize,
    kind: String,
}

fn cache_file() -> PathBuf {
    let home = std::env::var("HOME").map_or_else(
        |_| {
            std::env::var("USERPROFILE").map_or_else(|_| PathBuf::from("."), PathBuf::from)
        },
        PathBuf::from,
    );
    home.join(".kavach").join("origin_cache.json")
}

fn load_cache() -> HashMap<String, CacheEntry> {
    let path = cache_file();
    let Ok(data) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_cache(cache: &HashMap<String, CacheEntry>) {
    let path = cache_file();
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!("warning: failed to create cache dir: {e}");
    }
    if let Ok(json) = serde_json::to_string_pretty(cache)
        && let Err(e) = fs::write(path, json)
    {
        eprintln!("warning: failed to write cache: {e}");
    }
}

pub(super) fn get(name: &str, root: &Path) -> Option<Site> {
    let cache = load_cache();
    let entry = cache.get(name)?;
    let full_path = root.join(&entry.file);
    if !full_path.exists() {
        return None;
    }
    let kind = Kind::from_label(&entry.kind)?;
    Some(Site {
        kind,
        file: entry.file.clone(),
        line: entry.line,
    })
}

pub(super) fn put(name: &str, site: &Site) {
    let mut cache = load_cache();
    if cache.len() >= MAX_CACHE_ENTRIES && !cache.contains_key(name)
        && let Some(key) = cache.keys().next().cloned()
    {
        cache.remove(&key);
    }
    cache.insert(
        name.to_owned(),
        CacheEntry {
            file: site.file.clone(),
            line: site.line,
            kind: site.kind.label().to_owned(),
        },
    );
    save_cache(&cache);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_put_and_get() {
        let temp_dir = std::env::temp_dir();
        let site = Site {
            kind: Kind::Function,
            file: "Cargo.toml".to_owned(),
            line: 1,
        };
        put("test_symbol", &site);
        let _retrieved = get("test_symbol", &temp_dir);
        // "Cargo.toml" may not exist in temp_dir, but get checks full_path.exists()
        // If we test with a guaranteed path:
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let retrieved = get("test_symbol", root);
        assert!(retrieved.is_some());
        let s = retrieved.unwrap();
        assert_eq!(s.line, 1);
        assert_eq!(s.kind, Kind::Function);
    }
}
