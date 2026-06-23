// split: RPC namespace for implementation-flow DAG verbs (flow.upsert / flow.render).
// Mirrors methods/concept.rs: DTOs at the boundary, delegate to kavach_surreal
// graph_* helpers, map errors via surreal_to_rpc. The DAG is the store; Mermaid
// is rendered on read by walking it.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    FlowDag, FlowEdgeInput, FlowSpec, FlowStepInput, graph_fetch_flow, graph_list_flows,
    graph_upsert_flow,
};
use serde::{Deserialize, Serialize};

/// Parameters for `db.flow_upsert`.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct UpsertParams {
    /// Project slug the flow belongs to.
    pub project_slug: String,
    /// Project-scoped flow key.
    pub flow_key: String,
    /// Display title.
    pub flow_title: String,
    /// Steps (nodes).
    pub steps: Vec<FlowStepInput>,
    /// Dependency edges (`from` is prerequisite of `to`).
    pub edges: Vec<FlowEdgeInput>,
    /// Optional raw Mermaid source cached for round-trip.
    #[serde(default)]
    pub raw_mermaid: Option<String>,
}

/// Result of `db.flow_upsert`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct UpsertResult {
    /// Record id of the flow anchor entity.
    pub flow_id: String,
    /// Number of steps persisted.
    pub step_count: usize,
}

/// Parameters for `db.flow_render`.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct RenderParams {
    /// Project slug.
    pub project_slug: String,
    /// Flow key.
    pub flow_key: String,
    /// `"mermaid"` (default) or `"json"`.
    #[serde(default)]
    pub format: Option<String>,
}

/// Result of `db.flow_render`: exactly one of `mermaid` / `json` is set.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct RenderResult {
    /// Rendered Mermaid `flowchart TD`, when `format == "mermaid"`.
    pub mermaid: Option<String>,
    /// The full DAG, when `format == "json"`.
    pub dag: Option<FlowDag>,
}

/// Upsert an implementation-flow DAG. Idempotent on `(project_slug, flow_key)`.
///
/// # Errors
/// Returns `ErrorObjectOwned` when the project is unregistered, the edges form
/// a cycle / reference an unknown step, or the database operation fails.
pub async fn upsert(state: &AppState, p: UpsertParams) -> Result<UpsertResult, ErrorObjectOwned> {
    let step_count = p.steps.len();
    let spec = FlowSpec {
        flow_key: p.flow_key,
        flow_title: p.flow_title,
        steps: p.steps,
        edges: p.edges,
        raw_mermaid: p.raw_mermaid,
    };
    let id = graph_upsert_flow(&state.db, &p.project_slug, &spec)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(UpsertResult {
        flow_id: format!("{id:?}"),
        step_count,
    })
}

/// Parameters for `db.flow_list`.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct ListParams {
    /// Project slug to list flows for.
    pub project_slug: String,
}

/// One flow's identity in a list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct FlowSummary {
    /// Flow key.
    pub flow_key: String,
    /// Flow title.
    pub flow_title: String,
}

/// List the flows defined for a project (key + title only).
///
/// # Errors
/// Returns `ErrorObjectOwned` on database failure.
pub async fn list(state: &AppState, p: ListParams) -> Result<Vec<FlowSummary>, ErrorObjectOwned> {
    let flows = graph_list_flows(&state.db, &p.project_slug)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(flows
        .into_iter()
        .map(|(flow_key, flow_title)| FlowSummary {
            flow_key,
            flow_title,
        })
        .collect())
}

/// Render a stored flow as Mermaid (default) or JSON by walking its DAG.
///
/// # Errors
/// Returns `ErrorObjectOwned` when the project or flow does not exist, or the
/// database operation fails.
pub async fn render(state: &AppState, p: RenderParams) -> Result<RenderResult, ErrorObjectOwned> {
    let dag = graph_fetch_flow(&state.db, &p.project_slug, &p.flow_key)
        .await
        .map_err(surreal_to_rpc)?;
    let fmt = p.format.as_deref().unwrap_or("mermaid");
    if fmt == "json" {
        Ok(RenderResult {
            mermaid: None,
            dag: Some(dag),
        })
    } else {
        Ok(RenderResult {
            mermaid: Some(dag.to_mermaid()),
            dag: None,
        })
    }
}

/// Parameters for `db.decision_render` — the `DECISION_MAP` architecture graph.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct DecisionRenderParams {
    /// Project slug whose decision architecture to render.
    pub project_slug: String,
    /// Optional focus keys (qnames or bare keys) to restrict the neighbourhood;
    /// empty renders the whole decision spine.
    #[serde(default)]
    pub focus: Vec<String>,
    /// Max nodes to keep (token discipline); defaults to 8.
    #[serde(default)]
    pub max_nodes: Option<usize>,
}

/// Result of `db.decision_render`: the Mermaid `graph TD`, or `None` when the
/// project has no decision/roadmap nodes (nothing to inject).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct DecisionRenderResult {
    /// Rendered status-styled `graph TD`, or `None` when empty.
    pub mermaid: Option<String>,
}

/// Default node cap for the decision map (token discipline).
const DECISION_MAP_DEFAULT_CAP: usize = 8;

/// Render the decision-architecture slice of a project's graph as Mermaid.
///
/// # Errors
/// Returns `ErrorObjectOwned` on database failure (project missing ⇒ empty graph
/// ⇒ `mermaid: None`, not an error).
pub async fn decision_render(
    state: &AppState,
    p: DecisionRenderParams,
) -> Result<DecisionRenderResult, ErrorObjectOwned> {
    let dag = kavach_surreal::roadmap_dag_fetch(&state.db, &p.project_slug)
        .await
        .map_err(surreal_to_rpc)?;
    let cap = p.max_nodes.unwrap_or(DECISION_MAP_DEFAULT_CAP);
    Ok(DecisionRenderResult {
        mermaid: dag.decision_mermaid(&p.focus, cap),
    })
}

/// Parameters for `db.practice_render` — the `PRACTICE_DELTA` worst-vs-best graph.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct PracticeRenderParams {
    /// Max anti-patterns to contrast (token discipline); defaults to 6.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Result of `db.practice_render`: the Mermaid `graph LR`, or `None` when the
/// ledger holds no anti-patterns (nothing to contrast).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct PracticeRenderResult {
    /// Rendered worst-vs-best `graph LR`, or `None` when empty.
    pub mermaid: Option<String>,
}

/// Default anti-pattern cap for the practice delta (token discipline).
const PRACTICE_DELTA_DEFAULT_CAP: usize = 6;

/// Render the recurrence-ranked worst-vs-best practice contrast as Mermaid.
///
/// # Errors
/// Returns `ErrorObjectOwned` on database failure (empty ledger ⇒ `mermaid: None`).
pub async fn practice_render(
    state: &AppState,
    p: PracticeRenderParams,
) -> Result<PracticeRenderResult, ErrorObjectOwned> {
    let cap = p.limit.unwrap_or(PRACTICE_DELTA_DEFAULT_CAP);
    let ranked = kavach_surreal::graph_top_anti_patterns(&state.db, cap)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(PracticeRenderResult {
        mermaid: kavach_surreal::practice_delta_mermaid(&ranked),
    })
}

/// Parameters for `db.stack_render` — the `STACK_FIT` language/stack invariant graph.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct StackRenderParams {
    /// Project slug whose stack invariants to render.
    pub project_slug: String,
}

/// Result of `db.stack_render`: the Mermaid `graph TD`, or `None` when the
/// project declares no `stack.*` `app_spec` rows (nothing to bind).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct StackRenderResult {
    /// Rendered `graph TD` binding each component to its boundary, or `None`.
    pub mermaid: Option<String>,
}

/// Render the project's chosen language/tech-stack invariants as Mermaid.
///
/// # Errors
/// Returns `ErrorObjectOwned` on database failure (project missing or no
/// `stack.*` rows ⇒ `mermaid: None`, not an error).
pub async fn stack_render(
    state: &AppState,
    p: StackRenderParams,
) -> Result<StackRenderResult, ErrorObjectOwned> {
    let invariants = kavach_surreal::stack_invariants(&state.db, &p.project_slug)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(StackRenderResult {
        mermaid: kavach_surreal::stack_fit_mermaid(&invariants),
    })
}

/// Parameters for `db.pattern_render` — the pattern supersession DAG.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct PatternRenderParams {
    /// Project slug whose pattern layer to render.
    pub project_slug: String,
    /// Optional focus keys (qnames or bare keys) to restrict the neighbourhood;
    /// empty renders the whole pattern layer.
    #[serde(default)]
    pub focus: Vec<String>,
    /// Max pattern nodes to keep (token discipline); defaults to 8.
    #[serde(default)]
    pub max_nodes: Option<usize>,
}

/// Result of `db.pattern_render`: the Mermaid `graph TD`, or `None` when the
/// project has no pattern nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct PatternRenderResult {
    /// Rendered supersession `graph TD`, or `None` when empty.
    pub mermaid: Option<String>,
}

/// Default pattern-node cap (token discipline).
const PATTERN_DAG_DEFAULT_CAP: usize = 8;

/// Render the research-refreshed pattern layer's supersession DAG as Mermaid.
///
/// # Errors
/// Returns `ErrorObjectOwned` on database failure (project missing or no pattern
/// rows ⇒ `mermaid: None`, not an error).
pub async fn pattern_render(
    state: &AppState,
    p: PatternRenderParams,
) -> Result<PatternRenderResult, ErrorObjectOwned> {
    let dag = kavach_surreal::roadmap_dag_fetch(&state.db, &p.project_slug)
        .await
        .map_err(surreal_to_rpc)?;
    let cap = p.max_nodes.unwrap_or(PATTERN_DAG_DEFAULT_CAP);
    Ok(PatternRenderResult {
        mermaid: dag.pattern_dag_mermaid(&p.focus, cap),
    })
}

/// Parameters for `db.retired_patterns` — the enforcement-teeth lookup.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct RetiredPatternsParams {
    /// Project slug whose retired patterns to fetch.
    pub project_slug: String,
}

/// One retired pattern and what replaced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct RetiredPattern {
    /// Title of the pattern the codebase retired.
    pub retired: String,
    /// Title of the pattern that replaced it.
    pub replacement: String,
}

/// Fetch the retired patterns (supersession edge targets) for a project.
///
/// # Errors
/// Returns `ErrorObjectOwned` on database failure (project missing ⇒ empty list).
pub async fn retired_patterns(
    state: &AppState,
    p: RetiredPatternsParams,
) -> Result<Vec<RetiredPattern>, ErrorObjectOwned> {
    let dag = kavach_surreal::roadmap_dag_fetch(&state.db, &p.project_slug)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(dag
        .retired_patterns()
        .into_iter()
        .map(|(retired, replacement)| RetiredPattern {
            retired,
            replacement,
        })
        .collect())
}
