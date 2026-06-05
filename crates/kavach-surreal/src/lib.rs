// hub: crate root — module declarations + public re-export surface only, no logic.
// A re-export hub aggregates the crate's public API; it cannot decompose without a
// mod.rs (which the micro-file rule forbids), so it carries the hub exemption.
// SOURCE: https://rust-lang.github.io/rust-clippy/rust-1.94.0/index.html#result_large_err
// Reason: surrealdb::Error is enum-driven and large by design; boxing the entire
// `Result<T, Error>` chain would touch every public API. Pre-existing latent lints
// surfaced by deploy.rs warnings-as-errors gate (TASK#1297).
#![allow(
    clippy::result_large_err,
    reason = "TASK#1297: surrealdb::Error is large; boxing requires API-wide refactor"
)]
#![allow(
    clippy::should_implement_trait,
    reason = "TASK#1297: Graph::from_str predates std::str::FromStr; rename is a public API break"
)]

pub mod archive;
pub mod bulk_manifest;
pub mod connection;
pub mod decisions;
pub mod delete;
pub mod bandit;
pub mod dual_write;
pub mod embed;
pub mod error;
pub mod filter;
pub mod gate_patterns;
pub mod graph;
pub mod harness_link;
pub mod lease;
pub mod parts;
pub mod projects;
pub mod rag_trees;
pub mod read;
pub mod schema;
pub mod schema_engine;
pub mod schema_v2;
pub mod session_store;
pub mod wipe;
pub mod write;

pub use connection::{default_db_path, open_db, open_default, open_default_daemon, open_memory};
pub use dual_write::MemoryEntry;
pub use embed::{EMBED_DIM, Embedder, cosine};
pub use error::{Error, Result};
pub use graph::upsert_relationships;
pub use graph::{DagEdge, DagNode, RoadmapDag, roadmap_dag_fetch};
pub use graph::{Edge, Entity, RelateParams, RelationType};
pub use graph::{LinkedRow, list_rows_with_links};
pub use graph::{
    RelatedRow, find_entity as graph_find_entity, get_related as graph_get_related,
    list_entities as graph_list_entities, relate_dynamic as graph_relate_dynamic,
    upsert_entity as graph_upsert_entity,
};
pub use graph::{backward, create_entity, delete_edge, forward, get_entity, relate};
// L0 concept tier RPCs
pub use graph::{
    delete_concept as graph_delete_concept,
    delete_concepts_by_prefix as graph_delete_concepts_by_prefix,
    find_concept as graph_find_concept, list_concepts as graph_list_concepts,
    relate_concepts as graph_relate_concepts, search_concepts_fts as graph_search_concepts_fts,
    upsert_concept as graph_upsert_concept,
};
// L1->L0 bridge RPCs
pub use graph::{
    BridgeHit, ProjectHit, bridge_to_concept as graph_bridge_to_concept,
    concepts_for_project as graph_concepts_for_project,
    projects_for_concept as graph_projects_for_concept,
};
// L3 mistake-event RPCs
pub use archive::{ArchiveReport, archive_irrelevant};
pub use decisions::{
    AlgoDecision, AlgoUpsertParams, ArchDecision, ArchUpsertParams, algo_list_recent, algo_upsert,
    arch_list_recent, arch_upsert,
};
pub use delete::{
    DeleteReport, delete_by_key, delete_category, preview_delete_by_key, preview_delete_category,
};
pub use filter::{FilterBuilder, FilterExpr, FilterValue};
pub use gate_patterns::{
    GatePattern, UpsertParams as GatePatternUpsertParams,
    find_autonomous as gate_pattern_find_autonomous, list_hot as gate_pattern_list_hot,
    tokenize as gate_pattern_tokenize, upsert as gate_pattern_upsert,
};
pub use graph::{
    append_mistake_event as graph_append_mistake_event,
    cluster_event_to_pattern as graph_cluster_event_to_pattern,
    query_anti_pattern_hit_count as graph_query_anti_pattern_hit_count,
    upsert_anti_pattern as graph_upsert_anti_pattern,
};
pub use harness_link::{GoalAttempt, latest_goal_attempt, set_harness};
pub use parts::{
    Part, find_by_path as part_find_by_path, list_by_project as parts_list_by_project,
    upsert as part_upsert,
};
pub use projects::{
    Project, ProjectNode, assemble_forest as projects_assemble_forest,
    build_forest as projects_build_forest, find_by_path as project_find_by_path,
    get_ancestry as project_get_ancestry, get_by_slug as project_get_by_slug,
    list_all as projects_list_all, register as project_register,
    relative_to_parent as project_relative_to_parent, set_parent as project_set_parent,
};
pub use rag_trees::{
    RagTreeLabel, RagTreeRefreshable, RagTreeRow, get as rag_tree_get, list as rag_tree_list,
    list_refreshable as rag_tree_list_refreshable, upsert_with_dir as rag_tree_upsert_with_dir,
};
pub use read::{
    get_by_id, get_by_key, list_all_by_table, list_by_project, list_by_status, list_with_filter,
};
pub use schema::apply_schema;
pub use schema_engine::apply as apply_schema_engine;
pub use schema_v2::apply_agent_memory_schema;
pub use session_store::{SessionRuntimeRow, session_get_by_id, session_upsert};
pub use wipe::{WipeReport, preview_wipe, wipe_project};
pub use bandit::{
    append_bandit_row, list_bandit_rows, list_unrewarded_bandit_rows,
    list_unrewarded_bandit_rows_for_session, update_bandit_reward,
};
pub use write::{
    ExpireReport, append_event, expire_stale, rotate_events, set_priority, update_feedback,
    update_status, upsert_entry, upsert_entry_full, upsert_entry_with_event,
};
