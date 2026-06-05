#![expect(
    clippy::multiple_inherent_impl,
    reason = "Runner impl deliberately split across runner.rs + context_output.rs for file-size locality"
)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "nursery lint conflicts with workspace unreachable_pub=deny: pub(crate) fns in private modules satisfy unreachable_pub; redundant_pub_crate's pub suggestion would re-trigger it"
)]

pub mod chain_state;
pub mod context_output;
pub mod gates;
pub(crate) mod helpers;
pub mod intent_features;
pub mod intent_tree;
pub mod kprobe;
pub mod loader;
pub(crate) mod loader_types;
pub mod research_gate;
pub mod router;
pub mod runner;
pub mod stop_features;
pub mod stop_intent_tree;
pub mod stop_signals;
pub mod types;

pub use chain_state::ChainState;
pub use gates::aegis::aegis_verify;
pub use gates::ceo::ceo_validate;
pub use gates::intent::analyze_intent;
pub use gates::research::research_check;
pub use intent_features::extract_features;
pub use intent_tree::build_intent_tree;
pub use loader::DynamicLoader;
pub use loader_types::{AgentDef, SkillDef};
pub use research_gate::*;
pub use router::framework_detect::extract_framework_from_task;
pub use router::skill_first::SkillFirstRouter;
pub use router::types::RoutingDecision;
pub use runner::Runner;
pub use types::*;
