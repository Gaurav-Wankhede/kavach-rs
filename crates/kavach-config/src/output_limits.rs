use std::collections::HashMap;
// ALGO: parking_lot::Mutex (futex / queue-based parking)
// PROBLEM_CLASS: thread synchronisation (single-writer cache guard)
// REJECTED: [
//   {"name":"std::sync::Mutex","reason":"24-byte storage, OS-level pthread_mutex, 1.5x-5x slower"},
//   {"name":"tokio::sync::Mutex","reason":"async-only; this code is sync — would force runtime"},
//   {"name":"RwLock","reason":"read-and-write paths both mutate cache cell; reader-bias adds overhead"}
// ]
// TIME: lock O(1) uncontended | SPACE: 1 byte (vs 24 std)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: no poisoning — accept that a panic mid-mutation can leave cache inconsistent (we re-load anyway)
// BENCHMARK: https://amanieu.github.io/parking_lot/parking_lot/struct.Mutex.html
// SOURCE: https://docs.rs/parking_lot/latest/parking_lot/type.Mutex.html
use crate::cache::{TTL, load_patterns};
use parking_lot::Mutex;
use std::time::Instant;

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal/match DTO; non_exhaustive => E0639"
)]
pub struct OutputLimits {
    pub agent_limits: HashMap<String, usize>,
    pub max_parallel: usize,
    pub phase_multipliers: HashMap<String, f64>,
    pub overflow_action: String,
    pub file_fallback_dir: String,
}

impl Default for OutputLimits {
    fn default() -> Self {
        let mut m = HashMap::new();
        m.insert("early".into(), 1.0);
        m.insert("mid".into(), 0.75);
        m.insert("late".into(), 0.5);
        m.insert("critical".into(), 0.25);
        Self {
            agent_limits: HashMap::new(),
            max_parallel: 3,
            phase_multipliers: m,
            overflow_action: "truncate_with_summary".into(),
            file_fallback_dir: "/tmp/kavach-agent-output".into(),
        }
    }
}

static OUTPUT_LIMITS_CACHE: std::sync::LazyLock<Mutex<Option<(OutputLimits, Instant)>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

pub fn load_output_limits() -> OutputLimits {
    // parking_lot::Mutex never poisons — .lock() returns the guard directly, no Result.
    let mut cache = OUTPUT_LIMITS_CACHE.lock();
    if let Some((ref limits, ts)) = *cache
        && ts.elapsed() < TTL
    {
        return limits.clone();
    }
    let data = load_patterns("output-limits.toon");
    let mut limits = OutputLimits::default();
    if let Some(lines) = data.get("AGENT_OUTPUT_LIMITS") {
        for line in lines {
            if let Some((k, v)) = split_kv(line)
                && let Ok(n) = v.parse::<usize>()
            {
                limits.agent_limits.insert(k.to_owned(), n);
            }
        }
    }
    if let Some(lines) = data.get("CONCURRENCY") {
        for line in lines {
            if let Some((k, v)) = split_kv(line)
                && k == "max_parallel_agents"
                && let Ok(n) = v.parse::<usize>()
            {
                limits.max_parallel = n;
            }
        }
    }
    if let Some(lines) = data.get("PHASE_MULTIPLIERS") {
        for line in lines {
            if let Some((k, v)) = split_kv(line)
                && let Ok(f) = v.parse::<f64>()
            {
                limits.phase_multipliers.insert(k.to_owned(), f);
            }
        }
    }
    if let Some(lines) = data.get("OVERFLOW") {
        for line in lines {
            if let Some((k, v)) = split_kv(line) {
                match k {
                    "action" => v.clone_into(&mut limits.overflow_action),
                    "file_fallback_dir" => v.clone_into(&mut limits.file_fallback_dir),
                    _ => {}
                }
            }
        }
    }
    *cache = Some((limits.clone(), Instant::now()));
    limits
}

#[must_use]
pub fn split_kv(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(':')?;
    let left = line.get(..idx)?;
    let right = line.get(idx.saturating_add(1)..)?;
    Some((left.trim(), right.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_output_limits_default() {
        let limits = OutputLimits::default();
        assert_eq!(limits.max_parallel, 3);
        assert_eq!(limits.phase_multipliers.get("early"), Some(&1.0));
        assert_eq!(limits.phase_multipliers.get("critical"), Some(&0.25));
    }
    #[test]
    fn test_split_kv() {
        assert_eq!(split_kv("key: value"), Some(("key", "value")));
        assert_eq!(split_kv("a:b"), Some(("a", "b")));
        assert_eq!(split_kv("no_colon"), None);
        assert_eq!(split_kv("key:  spaced  "), Some(("key", "spaced")));
    }
}
