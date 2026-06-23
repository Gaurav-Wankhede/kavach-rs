//! TRINITY role classification + role→backend routing over the vendor pool.
//!
//! A roadmap unit's title is the cheap, available signal for which role it plays:
//! planning/spec/design → Thinker, verification → Verifier, everything else →
//! Worker. The [`RolePool`] then maps each role to a [`VendorBackend`] so the
//! scheduler can route Thinker work to a high-capability backend and Worker work
//! to a cost-efficient one — the Fugu/TRINITY lesson applied to a harness pool.
//!
//! SOURCE: decision.fugu-orchestration-layer · https://sakana.ai/trinity/
use kavach_surreal::graph::roadmap_dag::{DagNode, RoadmapDag};

use crate::team::{AgentRole, CommandBackend, VendorBackend};

/// Lower-cased substrings that mark a title as planning/design (Thinker).
const THINKER_MARKERS: [&str; 5] = ["[plan]", "plan ", "spec", "design", "decompose"];
/// Lower-cased substrings that mark a title as verification (Verifier).
const VERIFIER_MARKERS: [&str; 4] = ["verify", "3-witness", "three-witness", "audit"];

/// Classify a task title into its TRINITY role. Verifier wins over Thinker when
/// both match (a "verify the design" card is verification work).
#[must_use]
pub fn role_for_title(title: &str) -> AgentRole {
    let t = title.to_ascii_lowercase();
    if VERIFIER_MARKERS.iter().any(|m| t.contains(m)) {
        AgentRole::Verifier
    } else if THINKER_MARKERS.iter().any(|m| t.contains(m)) {
        AgentRole::Thinker
    } else {
        AgentRole::Worker
    }
}

/// Classify a DAG node by its title.
#[must_use]
pub fn role_for_node(node: &DagNode) -> AgentRole {
    role_for_title(&node.title)
}

/// Pair each dispatch key with the role its node's title implies. A key absent
/// from `dag` (should not happen for a planned batch) defaults to Worker.
#[must_use]
pub fn role_assignments(keys: &[String], dag: &RoadmapDag) -> Vec<(String, AgentRole)> {
    keys.iter()
        .map(|k| {
            let role = dag
                .nodes
                .iter()
                .find(|n| &n.id == k)
                .map_or(AgentRole::Worker, role_for_node);
            (k.clone(), role)
        })
        .collect()
}

/// Maps each TRINITY role to the vendor backend that runs it. Defaults follow
/// the cost/capability split: Thinker→cc (high-capability), Worker→codex
/// (cost-efficient), Verifier→cc (verification stays on the in-house-gated path).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RolePool {
    /// Backend for Thinker (decompose/plan) work.
    pub thinker: CommandBackend,
    /// Backend for Worker (execute) work.
    pub worker: CommandBackend,
    /// Backend for Verifier work.
    pub verifier: CommandBackend,
}

impl Default for RolePool {
    fn default() -> Self {
        Self {
            thinker: CommandBackend::cc(),
            worker: CommandBackend::codex(),
            verifier: CommandBackend::cc(),
        }
    }
}

impl RolePool {
    /// The backend assigned to `role`.
    #[must_use]
    pub fn backend_for(&self, role: AgentRole) -> &dyn VendorBackend {
        match role {
            AgentRole::Thinker => &self.thinker,
            AgentRole::Worker => &self.worker,
            AgentRole::Verifier => &self.verifier,
        }
    }
}

#[cfg(test)]
#[path = "roles_test.rs"]
mod roles_test;
