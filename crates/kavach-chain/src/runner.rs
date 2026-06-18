use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;

use crate::chain_state::ChainState;
use crate::helpers::debug_stderr;

//   {"name":"WalkDir + sort_by_key","reason":"sorts ALL entries; we only need max — single pass O(N) beats sort O(N log N)"},
//   {"name":"glob crate","reason":"adds dependency for prefix match that Path already supports; overkill"},
//   {"name":"fs::metadata mtime","reason":"mtime can lie under cp/touch; filename embeds the trusted Local::now().timestamp() the writer used"}
// ]
// TIME: O(N) single pass over directory entries
// SPACE: O(1) — only tracks (max_ts, path)
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: https://doc.rust-lang.org/std/fs/fn.read_dir.html

/// Load the most recent chain_*.json for `session_id` from `cache_dir`.
/// Returns None on `read_dir` failure or absence of any matching file — caller
/// falls back to fresh `ChainState`. Per-file errors (corrupt name, overflow,
/// unreadable file) are skipped, NOT propagated, so one bad neighbor cannot
/// kill the whole load. First-run behavior: `cache_dir` does not yet exist;
/// `fs::read_dir` returns Err which `.ok()?` converts to None, and `Runner::new`
/// falls through to `ChainState::new`. `save_state()` later creates the dir.
fn load_session_state(cache_dir: &Path, session_id: &str) -> Option<ChainState> {
    let prefix = format!("chain_{session_id}_");
    let entries = fs::read_dir(cache_dir).ok()?;
    let mut latest: Option<(i64, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with(&prefix) || !name_str.ends_with(".json") {
            continue;
        }
        // Per-file errors must `continue`, never early-return — one corrupt
        // filename in the cache directory must not erase the whole session.
        let ts: i64 = match name_str
            .trim_start_matches(&prefix)
            .trim_end_matches(".json")
            .parse::<i64>()
        {
            Ok(t) if t > 0 => t, // reject negative or zero — invalid Unix ts
            _ => continue,
        };
        match &latest {
            Some((best, _)) if *best >= ts => {}
            _ => latest = Some((ts, entry.path())),
        }
    }
    let path = latest?.1;
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at RPC handler boundary"
)]
pub struct Runner {
    pub state: ChainState,
    pub cache_dir: PathBuf,
    pub debug_mode: bool,
}

impl Runner {
    #[must_use]
    pub fn new(session_id: &str) -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let cache_dir = PathBuf::from(&home).join(".claude").join("chain");
        // Restore satisfied_gates from most-recent chain_*.json for this
        // session so gates remember what was already satisfied across turns.
        // Cures FP-storm. See decision:rca.gate_session_amnesia.
        let state = load_session_state(&cache_dir, session_id)
            .unwrap_or_else(|| ChainState::new(session_id));
        Self {
            state,
            cache_dir,
            debug_mode: std::env::var("KAVACH_DEBUG").unwrap_or_default() == "1",
        }
    }

    pub fn run_full(
        &mut self,
        prompt: &str,
        tool_name: &str,
        tool_input: &HashMap<String, serde_json::Value>,
        research_done: bool,
    ) -> &ChainState {
        self.debug("Starting verification chain");
        let probe = crate::kprobe::Probe::start(tool_name);

        crate::gates::intent::run_gate(&mut self.state, prompt);
        if self.state.is_blocked() {
            return self.finalize();
        }

        let agent_type = tool_input
            .get("subagent_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        crate::gates::ceo::run_gate(&mut self.state, tool_name, &agent_type);
        if self.state.is_blocked() {
            return self.finalize();
        }

        crate::gates::aegis::run_gate(&mut self.state, tool_name, tool_input);
        if self.state.is_blocked() {
            return self.finalize();
        }

        crate::gates::research::run_gate(&mut self.state, research_done, prompt, &agent_type);
        if self.state.is_blocked() {
            return self.finalize();
        }

        self.state.final_status = "approved".into();
        let report = probe.stop();
        if self.debug_mode {
            self.debug(&report.render_kernel_observed_block());
        }
        self.state.kernel_observed = Some(report.render_kernel_observed_block());
        self.finalize()
    }

    pub(crate) fn finalize(&self) -> &ChainState {
        self.save_state();
        &self.state
    }

    fn save_state(&self) {
        if self.cache_dir.as_os_str().is_empty() {
            return;
        }
        if let Err(e) = fs::create_dir_all(&self.cache_dir) {
            debug_stderr(&format!(
                "chain state not persisted: cannot create {}: {e}",
                self.cache_dir.display()
            ));
            return;
        }
        let filename = format!(
            "chain_{}_{}.json",
            self.state.session_id,
            Local::now().timestamp()
        );
        let path = self.cache_dir.join(filename);
        match serde_json::to_string_pretty(&self.state) {
            Ok(data) => {
                if let Err(e) = fs::write(&path, data) {
                    debug_stderr(&format!(
                        "chain state not persisted: write {} failed: {e}",
                        path.display()
                    ));
                }
            }
            Err(e) => debug_stderr(&format!("chain state not persisted: serialize failed: {e}")),
        }
    }

    pub(crate) fn debug(&self, msg: &str) {
        if self.debug_mode {
            debug_stderr(msg);
        }
    }
}
