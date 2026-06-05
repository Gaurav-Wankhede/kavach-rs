//! P1 advisories: long-polling (prefer SSE) and queue consumers without
//! idempotency (queues deliver at-least-once).
use super::super::platform_guard_msg::build_advisory;
use super::super::platform_guard_paths::is_test;
use super::is_infra_file;

/// `Some(advisory)` listing infra-protocol nudges, or None if clean.
pub(crate) fn format_advisory(file_path: &str, content: &str) -> Option<String> {
    if !is_infra_file(file_path) || is_test(file_path) {
        return None;
    }
    let lc = content.to_lowercase();
    let mut p1: Vec<(&str, &str)> = Vec::new();

    if lc.contains("longpoll") || lc.contains("long_poll") {
        p1.push((
            "AVOID_LONG_POLLING",
            "Replace long polling with SSE (EventSource) — lower latency, fewer connections.",
        ));
    }
    if (lc.contains("queue") || file_path.contains("consumer"))
        && (lc.contains("queue") || lc.contains("batch"))
        && !lc.contains("idempoten")
        && !lc.contains("dedup")
    {
        p1.push((
            "QUEUE_NO_IDEMPOTENCY",
            "Add idempotency key to queue consumer — message queues deliver at-least-once.",
        ));
    }
    if p1.is_empty() {
        return None;
    }
    Some(build_advisory("INFRA_PROTOCOL_GUARD", &p1))
}
