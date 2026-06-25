use std::sync::{LazyLock, Mutex};

mod types;
mod defaults;
#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;

pub use types::{AntiProdLevel, AntiProdResult, Config};

use defaults::load_defaults;

static CACHED_CONFIG: LazyLock<Mutex<Option<Config>>> = LazyLock::new(|| Mutex::new(None));

pub fn load() -> std::sync::MutexGuard<'static, Option<Config>> {
    let mut guard = CACHED_CONFIG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.is_none() {
        *guard = Some(build_config());
    }
    guard
}

pub fn reload() {
    let mut g = CACHED_CONFIG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *g = Some(build_config());
}

fn build_config() -> Config {
    let mut cfg = Config {
        sensitive: Vec::new(),
        blocked: Vec::new(),
        code_exts: Vec::new(),
        large_exts: Vec::new(),
        valid_agents: std::collections::HashMap::new(),
        intent_words: std::collections::HashMap::new(),
        loaded_from: "defaults".into(),
    };
    load_defaults(&mut cfg);
    cfg
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) fn j(parts: &[&str]) -> String {
    parts.concat()
}
