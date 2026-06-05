#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at router boundary in skill_first"
)]
pub struct RoutingDecision {
    pub use_skill: bool,
    pub skill_name: String,
    pub agent_name: String,
    pub requires_ceo: bool,
    pub reason: String,
}
