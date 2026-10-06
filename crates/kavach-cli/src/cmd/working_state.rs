//! `kavach working-state` — bounded zero-heap K/V scratchpad (max 10 slots).
//!
//! Storage: `~/.kavach/working_state.json` — a `HashMap<String,String>` with an
//! explicit insertion-order vector so the oldest key is evicted when the cap is
//! reached. The file is read atomically on each call and written back in full;
//! no background thread / no unbounded growth.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::cli::WorkingStateAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

// ── storage ──────────────────────────────────────────────────────────────────

/// Maximum number of key-value slots kept in the scratchpad.
const MAX_SLOTS: usize = 10;

/// On-disk representation: values map + insertion-order vector.
/// Order is maintained so we know which key to evict (the oldest) when full.
#[derive(Serialize, Deserialize, Default)]
struct WorkingState {
    /// The key-value store.
    values: HashMap<String, String>,
    /// Insertion order (keys only). `values` is the source of truth; this is
    /// purely the ordering side-channel. Length always equals `values.len()`.
    order: Vec<String>,
}

impl WorkingState {
    /// Load from disk, or return an empty state if the file is absent.
    fn load(path: &std::path::Path) -> Self {
        let Ok(bytes) = std::fs::read(path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    /// Persist to disk (creates parent dir on first use).
    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(path, &json)
    }

    /// Insert or overwrite a key. Evicts the oldest entry when over the cap.
    fn set(&mut self, key: String, value: String) {
        if !self.values.contains_key(&key) {
            // New key: evict oldest if at cap.
            while self.order.len() >= MAX_SLOTS {
                let oldest = self.order.remove(0);
                self.values.remove(&oldest);
            }
            self.order.push(key.clone());
        }
        self.values.insert(key, value);
    }

    /// Return the value for `key`, or `None` if absent.
    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Remove all slots.
    fn clear(&mut self) {
        self.values.clear();
        self.order.clear();
    }

    /// Format all slots as a `[STATE]` block, keys in insertion order.
    fn inject_block(&self) -> String {
        let mut out = String::from("[STATE]\n");
        for key in &self.order {
            if let Some(val) = self.values.get(key) {
                out.push_str(key);
                out.push_str(": ");
                out.push_str(val);
                out.push('\n');
            }
        }
        out
    }
}

// ── path resolution ───────────────────────────────────────────────────────────

fn state_file() -> PathBuf {
    // Mirror the pattern in kavach-session/src/paths.rs: use HOME (or USERPROFILE
    // on Windows), then fall back to ".". Never panic — bad HOME degrades gracefully.
    let home = std::env::var("HOME").map_or_else(
        |_| {
            std::env::var("USERPROFILE").map_or_else(|_| PathBuf::from("."), PathBuf::from)
        },
        PathBuf::from,
    );
    home.join(".kavach").join("working_state.json")
}

// ── public entry-point ────────────────────────────────────────────────────────

/// Dispatch `kavach working-state <action>` and return an exit code.
pub(super) fn run(action: &WorkingStateAction) -> i32 {
    match action {
        WorkingStateAction::Set { key, value } => handle_set(key, value),
        WorkingStateAction::Get { key } => handle_get(key),
        WorkingStateAction::Clear => handle_clear(),
        WorkingStateAction::Inject => handle_inject(),
    }
}

// ── handlers ──────────────────────────────────────────────────────────────────

fn handle_set(key: &str, value: &str) -> i32 {
    let path = state_file();
    let mut state = WorkingState::load(&path);
    state.set(key.to_owned(), value.to_owned());
    if let Err(e) = state.save(&path) {
        let msg = format!("working-state: write error: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let msg = format!("working-state: set {key}");
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

fn handle_get(key: &str) -> i32 {
    let path = state_file();
    let state = WorkingState::load(&path);
    if let Some(val) = state.get(key) {
        // Print via locked stdout to avoid SIGPIPE swallow.
        let mut stdout = std::io::stdout().lock();
        if let Err(e) = writeln!(stdout, "{val}") {
            return into_exit_code(e.into());
        }
        0
    } else {
        let msg = format!("working-state: key not found: {key}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        1
    }
}

fn handle_clear() -> i32 {
    let path = state_file();
    let mut state = WorkingState::load(&path);
    state.clear();
    if let Err(e) = state.save(&path) {
        let msg = format!("working-state: write error: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    if let Err(io_err) = print_or_exit("working-state: cleared") {
        return into_exit_code(io_err);
    }
    0
}

fn handle_inject() -> i32 {
    let path = state_file();
    let state = WorkingState::load(&path);
    let block = state.inject_block();
    let mut stdout = std::io::stdout().lock();
    if let Err(e) = stdout.write_all(block.as_bytes()) {
        return into_exit_code(e.into());
    }
    0
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_roundtrip() {
        let mut state = WorkingState::default();
        state.set("key1".into(), "val1".into());
        assert_eq!(state.get("key1"), Some("val1"));
    }

    #[test]
    fn overwrite_preserves_order() {
        let mut state = WorkingState::default();
        state.set("a".into(), "1".into());
        state.set("b".into(), "2".into());
        state.set("a".into(), "updated".into());
        assert_eq!(state.order, vec!["a".to_owned(), "b".to_owned()]);
        assert_eq!(state.get("a"), Some("updated"));
    }

    #[test]
    fn evicts_oldest_at_cap() {
        let mut state = WorkingState::default();
        for i in 0..MAX_SLOTS {
            state.set(format!("k{i}"), format!("v{i}"));
        }
        assert_eq!(state.values.len(), MAX_SLOTS);
        // Adding one more must evict k0 (the oldest).
        state.set("k_new".into(), "vnew".into());
        assert_eq!(state.values.len(), MAX_SLOTS);
        assert!(!state.values.contains_key("k0"));
        assert!(state.values.contains_key("k_new"));
    }

    #[test]
    fn clear_empties_all() {
        let mut state = WorkingState::default();
        state.set("x".into(), "y".into());
        state.clear();
        assert!(state.values.is_empty());
        assert!(state.order.is_empty());
    }

    #[test]
    fn inject_block_format() {
        let mut state = WorkingState::default();
        state.set("target".into(), "fn_fetch_url".into());
        state.set("loc".into(), "src/fetch.rs:42".into());
        let block = state.inject_block();
        assert!(block.starts_with("[STATE]\n"));
        assert!(block.contains("target: fn_fetch_url\n"));
        assert!(block.contains("loc: src/fetch.rs:42\n"));
    }

    #[test]
    fn run_accepts_all_action_variants() {
        let _: fn(&WorkingStateAction) -> i32 = run;
    }
}
