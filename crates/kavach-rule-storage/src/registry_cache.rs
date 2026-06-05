//! Save, load, and check staleness of the registry cache.

use std::path::Path;

use crate::error::{Result, StorageError};
use crate::registry::SkillRegistry;

/// Save registry to disk as JSON.
///
/// # Errors
/// Returns [`StorageError`] if the file cannot be written or serialized.
pub fn save_registry(path: &Path, registry: &SkillRegistry) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(registry)
        .map_err(|e| StorageError::ParseError(e.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load registry from disk.
///
/// # Errors
/// Returns [`StorageError`] if the file cannot be read or deserialized.
pub fn load_registry(path: &Path) -> Result<SkillRegistry> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| StorageError::ParseError(e.to_string()))
}

#[must_use]
pub fn is_stale(cache_path: &Path, current_hash: &str) -> bool {
    match load_registry(cache_path) {
        Ok(cached) => cached.hash != current_hash,
        Err(_) => true,
    }
}

// ARCH: hash_based_cache_invalidation
// PATTERN: write-through cache with content-hash staleness
// REJECTED: [{"name":"mtime","reason":"unreliable across filesystems"},{"name":"polling","reason":"latency on cold start"}]
// INVARIANT: fresh.hash != cached.hash triggers rebuild
// TRADEOFF: reads skills_dir on every call; O(n) where n=skill files
// FAILURE_MODE: concurrent writes race; last-write-wins acceptable for local CLI
// CAPACITY: <100 skill files, <1ms rebuild latency
// MONITORING: none (CLI tool, no telemetry)
// SOURCE: Cargo fingerprint pattern

/// Load registry, auto-rebuilding from `skills_dir` if cache is stale or missing.
/// Uses content-hash comparison (not mtime) for reliable invalidation.
///
/// # Errors
/// Returns [`StorageError`] if rules cannot be loaded from the skills directory.
pub fn load_or_rebuild(cache_path: &Path, skills_dir: &Path) -> Result<SkillRegistry> {
    let rules = crate::loader::load_rules_from_dir(skills_dir)?;
    let fresh = crate::registry_builder::build_from_rules(&rules);

    match load_registry(cache_path) {
        Ok(cached) if cached.hash == fresh.hash => Ok(cached),
        Ok(_) | Err(_) => {
            // The cache write is best-effort — a failure does not invalidate the
            // freshly-built registry we return — but it must be OBSERVABLE, not
            // silently dropped: a persistently-failing write means every call
            // rebuilds from disk (a silent perf cliff) instead of hitting cache.
            if let Err(e) = save_registry(cache_path, &fresh) {
                // Best-effort cache write: a failure does not invalidate the
                // freshly-built registry we return, but it MUST be observable
                // (not silently dropped) — a persistently-failing write means
                // every call rebuilds from disk instead of hitting cache.
                tracing::warn!(
                    cache = %cache_path.display(),
                    error = %e,
                    "registry cache write failed; returning fresh registry, next load rebuilds"
                );
            }
            Ok(fresh)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{RegistryEntry, SkillRegistry};
    use kavach_rule_ast::SkillPriority;

    fn sample_registry(hash: &str) -> SkillRegistry {
        SkillRegistry {
            version: 1,
            hash: hash.into(),
            built_at: "2026-03-14T00:00:00".into(),
            skills: vec![RegistryEntry {
                name: "sp-rust".into(),
                file_patterns: vec!["*.rs".into()],
                priority: SkillPriority::Critical,
            }],
        }
    }

    #[test]
    fn test_save_and_load_registry() {
        let dir = tempfile::tempdir().expect("failed to create temp dir for registry test");
        let path = dir.path().join("registry.json");
        let reg = sample_registry("deadbeef");
        save_registry(&path, &reg).expect("save_registry should write JSON to temp path");
        let loaded = load_registry(&path).expect("load_registry should parse written JSON");
        assert_eq!(loaded.hash, "deadbeef");
        assert_eq!(loaded.skills.len(), 1);
        assert_eq!(loaded.version, 1);
    }

    #[test]
    fn test_load_missing_returns_err() {
        let result = load_registry(Path::new("/nonexistent/path/registry.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_is_stale_when_hash_differs() {
        let dir = tempfile::tempdir().expect("failed to create temp dir for staleness test");
        let path = dir.path().join("registry.json");
        let reg = sample_registry("hash-a");
        save_registry(&path, &reg).expect("save_registry should write registry for staleness test");
        assert!(is_stale(&path, "hash-b"));
        assert!(!is_stale(&path, "hash-a"));
    }
}
