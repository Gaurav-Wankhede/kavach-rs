use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use crate::cache::{TTL, dirs_home};
use crate::gates_config::GatesConfig;
use crate::gates_defaults::{default_gates_config, merge_gates_defaults};

static GATES_CACHE: std::sync::LazyLock<Mutex<Option<(GatesConfig, Instant)>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

fn gates_config_path() -> PathBuf {
    dirs_home()
        .join(".claude")
        .join("gates")
        .join("config.json")
}

/// Load gates config from ~/.claude/gates/config.json with TTL cache.
pub fn load_gates_config() -> GatesConfig {
    let mut cache = GATES_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((ref cfg, ts)) = *cache
        && ts.elapsed() < TTL
    {
        return cfg.clone();
    }
    let cfg = load_gates_config_from_file();
    *cache = Some((cfg.clone(), Instant::now()));
    cfg
}

fn load_gates_config_from_file() -> GatesConfig {
    let path = gates_config_path();
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            // Missing config file is expected on fresh installs — info, not
            // warn. Other I/O errors (perms, EIO) deserve operator attention.
            if e.kind() == std::io::ErrorKind::NotFound {
                tracing::debug!(
                    target: "kavach_config::gates_loader",
                    path = %path.display(),
                    "no gates config file; using built-in defaults"
                );
            } else {
                tracing::warn!(
                    target: "kavach_config::gates_loader",
                    path = %path.display(),
                    error = %e,
                    "failed to read gates config; falling back to built-in defaults"
                );
            }
            return default_gates_config();
        }
    };
    let mut cfg: GatesConfig = match serde_json::from_str(&data) {
        Ok(c) => c,
        Err(e) => {
            // Parse failure on a user-edited config is the dangerous case —
            // operator thinks their settings apply, but defaults silently win.
            // Surface at warn so it appears in production telemetry.
            tracing::warn!(
                target: "kavach_config::gates_loader",
                path = %path.display(),
                error = %e,
                "gates config JSON parse failed; falling back to built-in defaults"
            );
            return default_gates_config();
        }
    };
    merge_gates_defaults(&mut cfg);
    cfg
}

/// Reload gates config (force cache invalidation).
pub fn reload_gates_config() -> GatesConfig {
    let mut cache = GATES_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *cache = None;
    drop(cache);
    load_gates_config()
}

#[cfg(test)]
mod tests {
    use crate::gates_config::GatesConfig;
    use crate::gates_defaults::{default_gates_config, merge_gates_defaults};
    #[test]
    fn test_default_gates_config() {
        let cfg = default_gates_config();
        assert!(cfg.read.enabled);
        assert!(cfg.bash.enabled);
        assert!(cfg.write.enabled);
        assert!(!cfg.read.blocked_paths.is_empty());
        assert!(!cfg.bash.blocked_commands.is_empty());
    }
    #[test]
    fn test_merge_gates_defaults_fills_empty() {
        let mut cfg = GatesConfig::default();
        merge_gates_defaults(&mut cfg);
        assert!(!cfg.read.blocked_paths.is_empty());
        assert!(!cfg.bash.blocked_commands.is_empty());
    }
}
