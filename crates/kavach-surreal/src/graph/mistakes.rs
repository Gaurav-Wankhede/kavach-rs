// L0.5 anti_pattern + L3 mistake_event tiers in the same KG.
// Append-only events; hit-count is a count() query over inbound instance_of
// edges. Fixes the 3-bug RCA in mistake_ledger.rs at the substrate level.
// See decision/kg-iter2-pivot-mistakes-first.
pub mod append;
pub mod cluster;
pub mod count;
pub mod pattern;
pub mod policy;
pub mod policy_read;
pub mod top;

pub use append::append_mistake_event;
pub use cluster::cluster_event_to_pattern;
pub use count::query_anti_pattern_hit_count;
pub use pattern::upsert_anti_pattern;
pub use policy::{DeployedPolicyProps, upsert_deployed_policy};
pub use policy_read::{DeployedPolicyRow, top_deployed_policies};
pub use top::{AntiPatternRanked, top_anti_patterns};
