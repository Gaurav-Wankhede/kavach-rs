pub mod agent_gate;
pub mod dep_key;
pub mod status_check;

pub use agent_gate::is_agent_gated;
pub use dep_key::{dep_key_satisfied, parse_declared_deps};
pub use status_check::is_runnable_status;

/// Check if an entry is dispatchable to an agent.
///
/// True iff `entry` is dispatchable to an AGENT right now: every declared
/// `BLOCKED_BY/DEPENDS_ON` dep resolves to a verified/done row AND the card
/// is not agent-gated. A card with no declared deps and no gate is trivially ready.
/// `dep_pool` is the row set dependency KEYS resolve against — distinct from
/// the project-scoped candidate list. Dependency keys are a GLOBAL key space:
/// a card may legitimately declare a prerequisite owned by another project.
#[must_use]
pub fn deps_satisfied(
    entry: &kavach_surreal::MemoryEntry,
    dep_pool: &[kavach_surreal::MemoryEntry],
) -> bool {
    if is_agent_gated(&entry.title) || is_agent_gated(&entry.content) {
        return false;
    }
    parse_declared_deps(&entry.content)
        .iter()
        .all(|dep| dep_key_satisfied(dep, dep_pool))
}
