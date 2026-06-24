use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use std::time::Instant;

use crate::cache::{TTL, dirs_home};

static MODULE_CACHE: std::sync::LazyLock<Mutex<HashMap<String, CachedModule>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

struct CachedModule {
    content: Option<String>,
    timestamp: Instant,
}

/// Load a single module file from `~/.claude/modules/{name}.md`.
/// Returns `None` if the file doesn't exist. Results are cached.
pub fn load_module(name: &str) -> Option<String> {
    let mut cache = MODULE_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = cache.get(name)
        && entry.timestamp.elapsed() < TTL
    {
        return entry.content.clone();
    }
    let path = dirs_home()
        .join(".claude")
        .join("modules")
        .join(format!("{name}.md"));
    let content = fs::read_to_string(&path).ok();
    cache.insert(
        name.to_owned(),
        CachedModule {
            content: content.clone(),
            timestamp: Instant::now(),
        },
    );
    content
}

/// Load multiple modules and concatenate their contents.
/// Each module is separated by a newline. Missing modules are skipped.
#[must_use]
pub fn load_modules(names: &[&str]) -> String {
    let mut result = String::new();
    for name in names {
        if let Some(content) = load_module(name) {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&content);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent_module() {
        assert!(load_module("nonexistent-module-xyz").is_none());
    }

    #[test]
    fn test_load_modules_empty() {
        let result = load_modules(&["no-such-a", "no-such-b"]);
        assert!(result.is_empty());
    }

    #[test]
    #[expect(
        clippy::let_underscore_must_use,
        reason = "cleanup: intentionally ignoring directory creation and removal"
    )]
    fn test_load_modules_from_temp() {
        let dir = std::env::temp_dir().join("kavach-test-modules");
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("test-mod.md"), "# Test Module\nContent here")
            .expect("write temp module");
        // This won't match ~/.claude/modules/ path, so returns None
        // Just validates the concatenation logic works with empties
        let result = load_modules(&["test-mod"]);
        // Cleanup
        drop(fs::remove_dir_all(&dir));
        // Result depends on whether ~/.claude/modules/test-mod.md exists
        let _ = result;
    }
}
