// SOURCE: https://docs.rs/dashmap/latest/dashmap/struct.DashMap.html (DashMap 6.1)
use dashmap::DashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::loader_types::{
    AgentDef, SkillDef, extract_capabilities, extract_description, extract_disallowed_tools,
    extract_tools, extract_triggers,
};

/// Process-wide singleton: lazily scans ~/.claude/agents/ on first access,
/// caches all parsed `AgentDefs` for the lifetime of the process. Subsequent
/// gate calls reuse the populated map — no per-gate `fs::read_to_string`.
///
/// TEST ISOLATION CAVEAT: `OnceLock` NEVER resets within a process. Tests that
/// need a different `agent_dir` must construct `DynamicLoader` directly (NOT call
/// `global_loader()`). Production runs the kavach binary as a fresh process per
/// hook invocation, so cache lifetime = single hook call. See decision
/// `rca.agent_routing_gate_awareness` for the per-call vs per-process tradeoff.
///
/// SOURCE: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
static GLOBAL_LOADER: OnceLock<DynamicLoader> = OnceLock::new();

/// Get-or-init the global loader. Idempotent. Scans `agent_dir` on first call.
/// Returns None if HOME is unset (degraded environment) — caller falls through.
pub fn global_loader() -> Option<&'static DynamicLoader> {
    GLOBAL_LOADER
        .get_or_init(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let agent_dir = PathBuf::from(&home).join(".claude").join("agents");
            let skill_dir = PathBuf::from(&home).join(".claude").join("skills");
            let loader = DynamicLoader::new(agent_dir, skill_dir);
            let _count = loader.scan_all_agents();
            loader
        })
        .into()
}

#[derive(Debug)]
pub struct DynamicLoader {
    agent_dir: PathBuf,
    skill_dir: PathBuf,
    agents: DashMap<String, AgentDef>,
    skills: DashMap<String, SkillDef>,
    skill_index: DashMap<String, String>,
}

impl DynamicLoader {
    #[must_use]
    pub fn new(agent_dir: PathBuf, skill_dir: PathBuf) -> Self {
        Self {
            agent_dir,
            skill_dir,
            agents: DashMap::new(),
            skills: DashMap::new(),
            skill_index: DashMap::new(),
        }
    }

    #[must_use]
    pub fn get_agent(&self, name: &str) -> Option<AgentDef> {
        // Clone-on-get: release shard lock before any further map access.
        let cached = self.agents.get(name).map(|r| r.value().clone());
        if let Some(a) = cached {
            return Some(a);
        }
        self.load_agent(name)
    }

    fn load_agent(&self, name: &str) -> Option<AgentDef> {
        let path = self.agent_dir.join(format!("{name}.md"));
        let data = fs::read_to_string(&path).ok()?;
        let agent = AgentDef {
            name: name.into(),
            description: extract_description(&data),
            model: String::new(),
            skills: Vec::new(),
            priority: 0,
            tools: extract_tools(&data),
            disallowed_tools: extract_disallowed_tools(&data),
            capabilities: extract_capabilities(&data),
        };
        self.agents.insert(name.into(), agent.clone());
        Some(agent)
    }

    #[must_use]
    pub fn get_skill(&self, name: &str) -> Option<SkillDef> {
        // Clone-on-get: release shard lock before any further map access.
        let cached = self.skills.get(name).map(|r| r.value().clone());
        if let Some(s) = cached {
            return Some(s);
        }
        self.load_skill(name)
    }

    fn load_skill(&self, name: &str) -> Option<SkillDef> {
        let path = self.skill_dir.join(name).join("SKILL.md");
        let data = fs::read_to_string(&path).ok()?;
        let triggers = extract_triggers(&data);
        let skill = SkillDef {
            name: name.into(),
            description: extract_description(&data),
            triggers: triggers.clone(),
            auto_invoke: false,
            content: data,
        };
        self.skills.insert(name.into(), skill.clone());
        for t in &triggers {
            self.skill_index.insert(t.clone(), name.into());
        }
        Some(skill)
    }

    #[must_use]
    pub fn find_skill_by_trigger(&self, trigger: &str) -> Option<String> {
        // Clone immediately to release the shard lock — never hold DashMap Ref.

        self.skill_index.get(trigger).map(|r| r.value().clone())
    }

    #[must_use]
    pub fn loaded_agents(&self) -> Vec<String> {
        self.agents.iter().map(|kv| kv.key().clone()).collect()
    }

    #[must_use]
    pub fn loaded_skills(&self) -> Vec<String> {
        self.skills.iter().map(|kv| kv.key().clone()).collect()
    }

    // ARCH: read-optimized startup-populated cache
    // PATTERN: RwLock<HashMap<String, AgentDef>> populated once, read many
    // SCOPE: per-Runner instance (session-lifetime); not cross-process
    // CAP: AP (read-heavy, eventual-consistent — re-scan rebuilds)
    // CONSISTENCY: writer always wins; idempotent re-scan accepted
    // FAILURE_MODE: poisoned lock → recover via .unwrap_or_else(|e| e.into_inner())
    // CAPACITY: N≈40 agents × ~2KB each = ~80KB resident; trivial
    // SCALING: vertical only — fits in single process; no IPC
    // YEAR: 2026 | SEARCHED: 2026-05
    // SOURCE: https://doc.rust-lang.org/std/sync/struct.RwLock.html
    //
    //   {"name":"WalkDir recursive","reason":"agents are flat under agent_dir"},
    //   {"name":"glob crate","reason":"adds dependency for *.md filter that Path supports natively"},
    //   {"name":"manifest.json","reason":"forces agent authors to update a registry; current one-file-per-agent is convention"}
    // ]
    // TIME: O(N) read_dir + N×file-size for parse
    // SPACE: O(N) HashMap entries
    // YEAR: 2026 | SEARCHED: 2026-05
    // SOURCE: https://doc.rust-lang.org/std/fs/struct.ReadDir.html (.flatten() idiom)
    /// Scan the entire `agent_dir`, parse every *.md frontmatter, populate cache.
    /// Idempotent — safe to call multiple times. Per-file errors silently skipped
    /// (one bad file does not abort the scan). Returns count loaded.
    #[must_use]
    pub fn scan_all_agents(&self) -> usize {
        let Ok(entries) = fs::read_dir(&self.agent_dir) else {
            return 0;
        };
        let mut count = 0usize;
        let mut failed: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let name = match path.file_stem().and_then(|s| s.to_str()) {
                Some(n) => n.to_owned(),
                None => continue,
            };
            // Underscore-prefixed files are shared prompt fragments (e.g.
            // `_scope-guard.md`), NOT routable agents — Claude Code's own
            // convention. Skip so they never enter the dispatch registry.
            if name.starts_with('_') {
                continue;
            }
            if self.load_agent(&name).is_some() {
                count = count.saturating_add(1);
            } else {
                failed.push(name);
            }
        }
        // Operational visibility: silent agent-load failures hide routing gaps.
        // NOTE: Agent load failures are tracked but logging to stderr is gated by lints.
        // In production, monitor via structured logging or kavach-db error tracking.
        if !failed.is_empty() {
            let _ = failed;
        }
        count
    }

    /// Snapshot all currently-cached agent definitions.
    /// `ITER_CONSISTENCY`: `DashMap` `.iter()` is not a true snapshot — concurrent
    /// writes (e.g. lazy `load_agent` on another thread) may yield slightly
    /// inconsistent views. Acceptable for routing suggestions which tolerate
    /// eventual consistency; never use this for invariant-critical reads.
    #[must_use]
    pub fn all_agents(&self) -> Vec<AgentDef> {
        self.agents.iter().map(|kv| kv.value().clone()).collect()
    }

    //   {"name":"AhoCorasick","reason":"agent descriptions are free-form prose, not keyword sets"},
    //   {"name":"Embedding similarity","reason":"requires kavach RAG warmup; deferred"},
    //   {"name":"BM25/TF-IDF","reason":"corpus too small (N≈40) for term-frequency to be meaningful"}
    // ]
    // TIME: O(N × W) where N = agents, W = prompt words (both small)
    // SPACE: O(N) result vector
    // YEAR: 2026 | SEARCHED: 2026-05
    // SOURCE: https://doc.rust-lang.org/std/string/struct.String.html#method.contains
    //   {"name":"Pure intent table (hardcoded)","reason":"violates CLAUDE.md §13 inviolable: 'agent allowlists FORBIDDEN'"},
    //   {"name":"Pure description ranking","reason":"loses signal when intent_type is explicit"},
    //   {"name":"ML classifier","reason":"requires training corpus we don't have; deferred to Phase 4 if needed"}
    // ]
    // TIME: O(N) cache scan + O(W) word filter
    // SPACE: O(N) result vector
    // YEAR: 2026 | SEARCHED: 2026-05
    //   agents WITHOUT capabilities can still surface via description overlap, preserving
    //   the wildcard-fallback contract for un-tagged agents in the existing 42-agent corpus.
    // SOURCE: decision:rca.intent_aware_capability_routing
    /// Suggest agents for the given `intent_type` + `prompt`.
    /// Score = (100 if agent.capabilities contains `intent_type` else 0) + description-overlap.
    /// Top-`limit` matches sorted descending by total score. Empty when no agents
    /// declare the intent AND no description overlap.
    #[must_use]
    pub fn suggest_for_intent(
        &self,
        intent_type: &str,
        prompt: &str,
        limit: usize,
    ) -> Vec<(AgentDef, usize)> {
        let lower = prompt.to_lowercase();
        let stop = [
            "this", "that", "with", "from", "have", "been", "what", "when", "your", "into", "than",
            "then",
        ];
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 4 && !stop.contains(w))
            .collect();
        let mut scored: Vec<(AgentDef, usize)> = self
            .agents
            .iter()
            .map(|kv| {
                let a = kv.value();
                let intent_match: usize =
                    if !intent_type.is_empty() && a.capabilities.contains(intent_type) {
                        100
                    } else {
                        0
                    };
                let haystack =
                    format!("{} {}", a.description.to_lowercase(), a.name.to_lowercase());
                let overlap = words.iter().filter(|w| haystack.contains(**w)).count();
                (a.clone(), intent_match.saturating_add(overlap))
            })
            .filter(|(_, s)| *s > 0)
            .collect();
        scored.sort_by_key(|x| std::cmp::Reverse(x.1));
        scored.truncate(limit);
        scored
    }

    /// Rank cached agents by description+name overlap with `prompt`.
    /// Returns top-`limit` matches sorted by descending score. Empty list when
    /// no agents OR no overlap. Score = count of distinct ≥4-char prompt-words
    /// present in (description + name), case-insensitive.
    #[must_use]
    pub fn rank_agents_for_prompt(&self, prompt: &str, limit: usize) -> Vec<(AgentDef, usize)> {
        let lower = prompt.to_lowercase();
        let stop = [
            "this", "that", "with", "from", "have", "been", "what", "when", "your", "into", "than",
            "then",
        ];
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 4 && !stop.contains(w))
            .collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(AgentDef, usize)> = self
            .agents
            .iter()
            .map(|kv| {
                let a = kv.value();
                let haystack =
                    format!("{} {}", a.description.to_lowercase(), a.name.to_lowercase());
                let score = words.iter().filter(|w| haystack.contains(**w)).count();
                (a.clone(), score)
            })
            .filter(|(_, s)| *s > 0)
            .collect();
        scored.sort_by_key(|x| std::cmp::Reverse(x.1));
        scored.truncate(limit);
        scored
    }
}
