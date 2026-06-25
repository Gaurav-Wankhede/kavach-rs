use crate::error::Result;
use serde::{Deserialize, Serialize};

use super::shape::{escape_label, sanitize_id, NodeShape};

/// One step in a flow as supplied by the caller (structured ingest).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc boundary); non_exhaustive => E0639"
)]
pub struct FlowStepInput {
    /// Caller-assigned id, unique within the flow (e.g. `"validate"`).
    pub step_id: String,
    /// Human label shown in the rendered node.
    pub label: String,
    /// Optional Mermaid shape hint; defaults to `Rect`.
    #[serde(default)]
    pub shape: Option<String>,
    /// Optional longer description (stored, not rendered into the node box).
    #[serde(default)]
    pub description: Option<String>,
}

/// One dependency edge: `from` is a prerequisite of `to`
/// (`from --depends_on--> to`; arrow points prerequisite → dependent).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc boundary); non_exhaustive => E0639"
)]
pub struct FlowEdgeInput {
    /// `step_id` of the prerequisite.
    pub from: String,
    /// `step_id` of the dependent.
    pub to: String,
}

/// A full flow definition for upsert.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc boundary); non_exhaustive => E0639"
)]
pub struct FlowSpec {
    /// Project-scoped key, unique per project (e.g. `"auth-flow"`).
    pub flow_key: String,
    /// Display title.
    pub flow_title: String,
    /// Steps (nodes).
    pub steps: Vec<FlowStepInput>,
    /// Dependency edges between steps.
    pub edges: Vec<FlowEdgeInput>,
    /// Optional raw Mermaid source cached for round-trip; never source of truth.
    #[serde(default)]
    pub raw_mermaid: Option<String>,
}

/// A resolved step node read back from the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FlowStep {
    /// Caller id within the flow.
    pub step_id: String,
    /// Display label.
    pub label: String,
    /// Shape hint.
    pub shape: NodeShape,
    /// Optional description.
    pub description: Option<String>,
}

/// The flow DAG read back from the graph: ordered steps + dependency edges.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FlowDag {
    /// Flow key.
    pub flow_key: String,
    /// Flow title.
    pub flow_title: String,
    /// Steps (nodes), in stored order.
    pub steps: Vec<FlowStep>,
    /// Dependency edges as `(from_step_id, to_step_id)`.
    pub edges: Vec<(String, String)>,
    /// Cached raw Mermaid source, if one was supplied at ingest.
    pub raw_mermaid: Option<String>,
}

impl FlowDag {
    /// Kahn topological sort over `depends_on` edges. Returns `Err(Cycle)` with
    /// the residual step ids when a cycle makes a full ordering impossible.
    pub(super) fn detect_cycle(&self) -> Option<Vec<String>> {
        use std::collections::{HashMap, VecDeque};
        let mut in_deg: HashMap<&str, usize> =
            self.steps.iter().map(|s| (s.step_id.as_str(), 0)).collect();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for (from, to) in &self.edges {
            if !in_deg.contains_key(from.as_str()) || !in_deg.contains_key(to.as_str()) {
                continue;
            }
            adj.entry(from.as_str()).or_default().push(to.as_str());
            let d = in_deg.entry(to.as_str()).or_insert(0);
            *d = d.saturating_add(1);
        }
        let mut queue: VecDeque<&str> = in_deg
            .iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(&k, _)| k)
            .collect();
        let mut seen = 0usize;
        while let Some(n) = queue.pop_front() {
            seen = seen.saturating_add(1);
            if let Some(succ) = adj.get(n) {
                for &m in succ {
                    if let Some(d) = in_deg.get_mut(m) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(m);
                        }
                    }
                }
            }
        }
        if seen == self.steps.len() {
            None
        } else {
            Some(
                in_deg
                    .iter()
                    .filter(|&(_, &d)| d > 0)
                    .map(|(&k, _)| k.to_owned())
                    .collect(),
            )
        }
    }

    /// Render this DAG as a Mermaid `flowchart TD`. Steps emit as shaped nodes;
    /// each dependency edge emits as `from --> to`. Node ids are sanitized so
    /// the output is always valid Mermaid even for hostile step ids/labels.
    #[must_use]
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("flowchart TD\n");
        for step in &self.steps {
            let id = sanitize_id(&step.step_id);
            let label = escape_label(&step.label);
            out.push_str("  ");
            out.push_str(&id);
            out.push_str(&step.shape.wrap(&label));
            out.push('\n');
        }
        for (from, to) in &self.edges {
            out.push_str("  ");
            out.push_str(&sanitize_id(from));
            out.push_str(" --> ");
            out.push_str(&sanitize_id(to));
            out.push('\n');
        }
        out
    }
}
