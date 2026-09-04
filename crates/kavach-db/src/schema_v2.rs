// ARCH: AgentMemorySchema — SurrealDB 3.0 patterns for self-improving agents
// SEARCHED: 2026-05
// SOURCE: https://arxiv.org/html/2603.10600v1 (Trajectory-Informed Memory)
// SOURCE: https://arxiv.org/html/2512.18950v1 (MACLA hierarchical procedural memory)
// SOURCE: https://github.com/surrealdb/agent-memory (official agent-memory schema)
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

/// Applies the agent memory schema v2 to the database.
///
/// # Errors
/// Propagates `Error::Surreal` when the schema definition query fails.
pub async fn apply_agent_memory_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(AGENT_MEMORY_DDL).await?;
    Ok(())
}

const AGENT_MEMORY_DDL: &str = r"
-- =============================================================================
-- AGENT MEMORY SCHEMA v2 (SurrealDB 3.0 patterns)
-- =============================================================================
-- Design principles:
-- 1. Temporal versioning: every write creates a version record (journal)
-- 2. Procedural memory: extracted workflows from successful trajectories
-- 3. Trajectory storage: full execution paths for learning
-- 4. Working memory: short-term TTL-based context
-- 5. Episodic memory: compressed session summaries
-- =============================================================================

-- -----------------------------------------------------------------------------
-- TRAJECTORY: Execution path capture for self-improvement
-- BENCHMARK: arxiv.org/html/2603.10600v1 (14.3% improvement on AppWorld)
-- -----------------------------------------------------------------------------
DEFINE TABLE IF NOT EXISTS trajectory SCHEMAFULL;
DEFINE FIELD project ON trajectory TYPE record<project>;
DEFINE FIELD session_id ON trajectory TYPE string;
DEFINE FIELD goal ON trajectory TYPE string;
DEFINE FIELD outcome ON trajectory TYPE string
    ASSERT $value IN ['success', 'partial', 'failure', 'abandoned'];
DEFINE FIELD steps ON trajectory TYPE array<object>;
DEFINE FIELD duration_ms ON trajectory TYPE int DEFAULT 0;
DEFINE FIELD tool_calls ON trajectory TYPE int DEFAULT 0;
DEFINE FIELD token_usage ON trajectory TYPE int DEFAULT 0;
DEFINE FIELD learnings ON trajectory TYPE option<array<string>>;
DEFINE FIELD error_chain ON trajectory TYPE option<array<string>>;
DEFINE FIELD created_at ON trajectory TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_trajectory_project ON trajectory FIELDS project;
DEFINE INDEX IF NOT EXISTS idx_trajectory_outcome ON trajectory FIELDS project, outcome;
DEFINE INDEX IF NOT EXISTS idx_trajectory_session ON trajectory FIELDS session_id;

-- -----------------------------------------------------------------------------
-- PROCEDURAL MEMORY: Extracted reusable workflows (MACLA pattern)
-- BENCHMARK: arxiv.org/html/2512.18950v1 (187 procedures from 2851 trajectories)
-- -----------------------------------------------------------------------------
DEFINE TABLE IF NOT EXISTS procedure SCHEMAFULL;
DEFINE FIELD project ON procedure TYPE option<record<project>>;
DEFINE FIELD name ON procedure TYPE string;
DEFINE FIELD description ON procedure TYPE string;
DEFINE FIELD trigger_pattern ON procedure TYPE string;
DEFINE FIELD steps ON procedure TYPE array<object>;
DEFINE FIELD success_count ON procedure TYPE int DEFAULT 0;
DEFINE FIELD failure_count ON procedure TYPE int DEFAULT 0;
DEFINE FIELD reliability_score ON procedure TYPE float DEFAULT 0.5;
DEFINE FIELD last_used_at ON procedure TYPE option<datetime>;
DEFINE FIELD source_trajectories ON procedure TYPE array<record<trajectory>> DEFAULT [];
DEFINE FIELD created_at ON procedure TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON procedure TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_procedure_name ON procedure FIELDS name UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_procedure_trigger ON procedure FIELDS trigger_pattern;
DEFINE INDEX IF NOT EXISTS idx_procedure_reliability ON procedure FIELDS reliability_score;

-- -----------------------------------------------------------------------------
-- ROADMAP VERSIONS: Temporal journal for roadmap changes
-- BENCHMARK: surrealdb.com/blog/surrealmx-in-memory-storage-with-time-travel
-- -----------------------------------------------------------------------------
DEFINE TABLE IF NOT EXISTS roadmap_version SCHEMAFULL;
DEFINE FIELD roadmap ON roadmap_version TYPE record<roadmap>;
DEFINE FIELD version ON roadmap_version TYPE int;
DEFINE FIELD entry_status ON roadmap_version TYPE string;
DEFINE FIELD title ON roadmap_version TYPE string;
DEFINE FIELD content ON roadmap_version TYPE string;
DEFINE FIELD changed_by ON roadmap_version TYPE string DEFAULT 'agent';
DEFINE FIELD change_reason ON roadmap_version TYPE option<string>;
DEFINE FIELD created_at ON roadmap_version TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_roadmap_version_roadmap ON roadmap_version FIELDS roadmap;
DEFINE INDEX IF NOT EXISTS idx_roadmap_version_num ON roadmap_version FIELDS roadmap, version;

-- -----------------------------------------------------------------------------
-- DECISION VERSIONS: Temporal journal for decision changes
-- -----------------------------------------------------------------------------
DEFINE TABLE IF NOT EXISTS decision_version SCHEMAFULL;
DEFINE FIELD decision ON decision_version TYPE record<decision>;
DEFINE FIELD version ON decision_version TYPE int;
DEFINE FIELD status ON decision_version TYPE string;
DEFINE FIELD title ON decision_version TYPE string;
DEFINE FIELD content ON decision_version TYPE string;
DEFINE FIELD changed_by ON decision_version TYPE string DEFAULT 'agent';
DEFINE FIELD change_reason ON decision_version TYPE option<string>;
DEFINE FIELD created_at ON decision_version TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_decision_version_decision ON decision_version FIELDS decision;

-- -----------------------------------------------------------------------------
-- WORKING MEMORY: Short-term context for current task (TTL-based)
-- BENCHMARK: mem0.ai/blog/state-of-ai-agent-memory-2026
-- -----------------------------------------------------------------------------
DEFINE TABLE IF NOT EXISTS working_memory SCHEMAFULL;
DEFINE FIELD project ON working_memory TYPE record<project>;
DEFINE FIELD session_id ON working_memory TYPE string;
DEFINE FIELD key ON working_memory TYPE string;
DEFINE FIELD value ON working_memory TYPE object;
DEFINE FIELD ttl_seconds ON working_memory TYPE int DEFAULT 3600;
DEFINE FIELD created_at ON working_memory TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_working_memory_session ON working_memory FIELDS session_id, key UNIQUE;

-- -----------------------------------------------------------------------------
-- EPISODIC MEMORY: Compressed past session summaries
-- BENCHMARK: gleecus.com/blogs/ai-agent-memory-intelligent-ai-agents-2026
-- -----------------------------------------------------------------------------
DEFINE TABLE IF NOT EXISTS episode SCHEMAFULL;
DEFINE FIELD project ON episode TYPE record<project>;
DEFINE FIELD session_id ON episode TYPE string;
DEFINE FIELD summary ON episode TYPE string;
DEFINE FIELD key_decisions ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD files_modified ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD errors_encountered ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD lessons_learned ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD duration_minutes ON episode TYPE int DEFAULT 0;
DEFINE FIELD turn_count ON episode TYPE int DEFAULT 0;
DEFINE FIELD created_at ON episode TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_episode_project ON episode FIELDS project;
DEFINE INDEX IF NOT EXISTS idx_episode_session ON episode FIELDS session_id UNIQUE;

-- -----------------------------------------------------------------------------
-- CONTEXT SNAPSHOT: Pre-computed aggregates for fast harness context
-- BENCHMARK: dev.to/uenyioha/writing-cli-tools-that-ai-agents-actually-want-to-use-39no
-- -----------------------------------------------------------------------------
DEFINE TABLE IF NOT EXISTS context_snapshot SCHEMAFULL;
DEFINE FIELD project ON context_snapshot TYPE record<project>;
DEFINE FIELD kanban_counts ON context_snapshot TYPE object;
DEFINE FIELD active_items ON context_snapshot TYPE array<object> DEFAULT [];
DEFINE FIELD recent_decisions ON context_snapshot TYPE array<object> DEFAULT [];
DEFINE FIELD hot_procedures ON context_snapshot TYPE array<object> DEFAULT [];
DEFINE FIELD computed_at ON context_snapshot TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_context_snapshot_project ON context_snapshot FIELDS project UNIQUE;
";
