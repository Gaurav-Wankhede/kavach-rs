//! Infrastructure Protocol Guard — enforces correct protocol usage.
//!
//! GraphQL: depth limit + disable introspection in production (`block`). Queues:
//! at-least-once delivery requires idempotency; long-polling → SSE (`advisory`).
mod advisory;
mod block;
#[cfg(test)]
#[path = "pre_write_infra_guard_test.rs"]
#[path = "pre_write_infra_guard_test.rs"]
mod tests;
pub(crate) use advisory::format_advisory;
pub(crate) use block::check;
// Split needles to avoid self-triggering when this file is scanned.
const GQL: &str = concat!("graph", "ql");
const INTRO: &str = concat!("intros", "pection");
const ENABLED: &str = concat!("enabl", "ed");
/// True for `.ts`/`.tsx`/`.rs` files (the infra-protocol surface).
fn is_infra_file(path: &str) -> bool {
    std::path::Path::new(path).extension().is_some_and(|e| {
        e.eq_ignore_ascii_case("ts")
            || e.eq_ignore_ascii_case("tsx")
            || e.eq_ignore_ascii_case("rs")
    })
}
