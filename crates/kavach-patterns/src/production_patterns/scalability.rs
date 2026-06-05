//! Scalability anti-patterns.

use super::types::{Severity, mk};
use crate::config::j;

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    let spwn = j(&["tok", "io::", "spa", "wn"]);

    vec![
        (
            mk(&format!(r"(?:loop|while)\s*\{{[^}}]*{spwn}")),
            "UNBOUNDED_SPAWN",
            "Spawn in loop — add semaphore limit",
            Severity::P0Critical,
        ),
        (
            mk(r"\.collect::<Vec<[^>]+>>\(\)"),
            "NO_CAPACITY",
            "collect() without capacity — use with_capacity if size known",
            Severity::P2Medium,
        ),
        (
            mk(r"std::thread::sleep|std::fs::read|std::fs::write"),
            "BLOCKING_ASYNC",
            "Blocking in async — use tokio equivalents",
            Severity::P0Critical,
        ),
        (
            mk(r"(?s)\.lock\(\)[^;]{0,100}\.await"),
            "LOCK_AWAIT",
            "Lock held across await — use tokio::sync::Mutex",
            Severity::P0Critical,
        ),
        (
            mk(r"(?s)for\s+\w+\s+in\s+.*\{[^}]*\.contains\("),
            "LINEAR_LOOP",
            ".contains() in loop — use HashSet for O(1) lookup",
            Severity::P1High,
        ),
        (
            mk(r"(?s)for\s+\w+\s+in\s+.*\{[^}]*(?:format!\(|String::from\()"),
            "ALLOC_LOOP",
            "String alloc in loop — preallocate or use Cow",
            Severity::P1High,
        ),
        (
            mk(r"mpsc::unbounded_channel|mpsc::channel\(\)"),
            "UNBOUNDED_CHANNEL",
            "Unbounded channel — use bounded with backpressure",
            Severity::P1High,
        ),
        (
            mk(r"(?:for|while|loop).*\.clone\(\).*\{"),
            "CLONE_HOT",
            "clone() in loop — use references or Cow",
            Severity::P2Medium,
        ),
    ]
}
