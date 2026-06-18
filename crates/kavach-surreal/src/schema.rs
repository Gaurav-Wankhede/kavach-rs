//! kavach:micro-file-exempt — one monolithic `SCHEMA_DDL` string constant.
//!
//! The body is a single `SurrealDB` DDL data literal that cannot decompose into
//! a hub+leaf module hierarchy; the LOC ceiling does not apply.
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

/// Applies the schema DDL to the `SurrealDB` instance.
///
/// # Errors
/// Propagates errors from the `SurrealDB` query execution.
pub async fn apply_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(SCHEMA_DDL).await?;
    Ok(())
}

const SCHEMA_DDL: &str = r#"
-- Projects (key-value style with record IDs)
DEFINE TABLE project SCHEMAFULL;
DEFINE FIELD slug ON project TYPE string ASSERT $value != NONE;
DEFINE FIELD display ON project TYPE string;
DEFINE FIELD workdir ON project TYPE option<string>;
DEFINE FIELD stack ON project TYPE option<string>;
DEFINE FIELD aliases ON project TYPE option<array<string>>;
DEFINE FIELD parent ON project TYPE option<record<project>>;
DEFINE FIELD created_at ON project TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON project TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_project_slug ON project FIELDS slug UNIQUE;

-- Sessions (document store with temporal)
DEFINE TABLE session SCHEMAFULL;
DEFINE FIELD project ON session TYPE record<project>;
DEFINE FIELD model_id ON session TYPE option<string>;
DEFINE FIELD started_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD ended_at ON session TYPE option<datetime>;
DEFINE FIELD turn_count ON session TYPE int DEFAULT 0;
DEFINE FIELD compact_count ON session TYPE int DEFAULT 0;
DEFINE FIELD context_phase ON session TYPE string DEFAULT 'early';
DEFINE FIELD token_budget_total ON session TYPE int DEFAULT 1000000;
DEFINE FIELD token_budget_used ON session TYPE int DEFAULT 0;
DEFINE INDEX idx_session_project ON session FIELDS project;

-- Decision (typed table, replaces memory_entries category='decision')
DEFINE TABLE decision SCHEMAFULL;
DEFINE FIELD project ON decision TYPE record<project>;
DEFINE FIELD entry_key ON decision TYPE string;
-- FIX: [config_drift] 3.0 SCHEMAFULL rejects undeclared fields the write
-- SETs (2.x silently dropped them). write.rs sets category=<table>;
-- declare it. SOURCE: surrealdb.com/docs DEFINE FIELD.
DEFINE FIELD OVERWRITE category ON decision TYPE string VALUE $value OR 'decision';
DEFINE FIELD title ON decision TYPE string;
DEFINE FIELD content ON decision TYPE string;
DEFINE FIELD status ON decision TYPE string DEFAULT 'active';
-- Knowledge-store rows record SETTLED facts, not work to dispatch; the work
-- queue is the `roadmap` table alone (kavach-db SKILL.md: next-task = roadmap+todo).
-- Default a new decision to `verified` so it never pollutes `search --status todo`
-- or the stop-gate census. (Was 'todo' -> 308 phantom decision "todos".)
DEFINE FIELD OVERWRITE entry_status ON decision TYPE string DEFAULT 'verified'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
DEFINE FIELD tags ON decision TYPE option<array<string>>;
DEFINE FIELD decay_score ON decision TYPE option<float>;
DEFINE FIELD access_count ON decision TYPE int DEFAULT 0;
DEFINE FIELD created_at ON decision TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON decision TYPE datetime DEFAULT time::now();
DEFINE FIELD accessed_at ON decision TYPE option<datetime>;
DEFINE INDEX idx_decision_project_key ON decision FIELDS project, entry_key UNIQUE;

-- Research (typed table, replaces memory_entries category='research')
DEFINE TABLE research SCHEMAFULL;
DEFINE FIELD project ON research TYPE record<project>;
DEFINE FIELD entry_key ON research TYPE string;
DEFINE FIELD OVERWRITE category ON research TYPE string VALUE $value OR 'research';
DEFINE FIELD title ON research TYPE string;
DEFINE FIELD content ON research TYPE string;
DEFINE FIELD source ON research TYPE option<string>;
DEFINE FIELD status ON research TYPE string DEFAULT 'active';
-- Research findings are cached SETTLED facts, not dispatchable work — default
-- `verified` (was 'todo' -> phantom research "todos"). Work queue = roadmap only.
DEFINE FIELD OVERWRITE entry_status ON research TYPE string DEFAULT 'verified'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
DEFINE FIELD decay_score ON research TYPE option<float>;
DEFINE FIELD access_count ON research TYPE int DEFAULT 0;
DEFINE FIELD created_at ON research TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON research TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_research_project_key ON research FIELDS project, entry_key UNIQUE;

-- Pattern (typed table, replaces memory_entries category='pattern')
DEFINE TABLE pattern SCHEMAFULL;
DEFINE FIELD project ON pattern TYPE record<project>;
DEFINE FIELD entry_key ON pattern TYPE string;
DEFINE FIELD OVERWRITE category ON pattern TYPE string VALUE $value OR 'pattern';
DEFINE FIELD title ON pattern TYPE string;
DEFINE FIELD content ON pattern TYPE string;
DEFINE FIELD status ON pattern TYPE string DEFAULT 'active';
-- Gate patterns are LEARNED facts (false-positive fixes), not dispatchable work —
-- default `verified` (was 'todo' -> phantom pattern "todos"). Work queue = roadmap.
DEFINE FIELD OVERWRITE entry_status ON pattern TYPE string DEFAULT 'verified'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
DEFINE FIELD decay_score ON pattern TYPE option<float>;
DEFINE FIELD access_count ON pattern TYPE int DEFAULT 0;
DEFINE FIELD created_at ON pattern TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON pattern TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_pattern_project_key ON pattern FIELDS project, entry_key UNIQUE;

-- Roadmap (typed table, replaces memory_entries category='roadmap')
DEFINE TABLE roadmap SCHEMAFULL;
DEFINE FIELD project ON roadmap TYPE record<project>;
DEFINE FIELD entry_key ON roadmap TYPE string;
DEFINE FIELD OVERWRITE category ON roadmap TYPE string VALUE $value OR 'roadmap';
DEFINE FIELD title ON roadmap TYPE string;
DEFINE FIELD content ON roadmap TYPE string;
DEFINE FIELD spec_key ON roadmap TYPE option<string>;
DEFINE FIELD status ON roadmap TYPE string DEFAULT 'active';
DEFINE FIELD OVERWRITE entry_status ON roadmap TYPE string DEFAULT 'todo'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
DEFINE FIELD decay_score ON roadmap TYPE option<float>;
DEFINE FIELD access_count ON roadmap TYPE int DEFAULT 0;
DEFINE FIELD created_at ON roadmap TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON roadmap TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_roadmap_project_key ON roadmap FIELDS project, entry_key UNIQUE;
DEFINE INDEX idx_roadmap_status ON roadmap FIELDS project, entry_status;

-- AppSpec (typed table, replaces memory_entries category='app_spec')
DEFINE TABLE app_spec SCHEMAFULL;
DEFINE FIELD project ON app_spec TYPE record<project>;
DEFINE FIELD entry_key ON app_spec TYPE string;
DEFINE FIELD OVERWRITE category ON app_spec TYPE string VALUE $value OR 'app_spec';
DEFINE FIELD title ON app_spec TYPE string;
DEFINE FIELD content ON app_spec TYPE string;
DEFINE FIELD status ON app_spec TYPE string DEFAULT 'active';
DEFINE FIELD OVERWRITE entry_status ON app_spec TYPE string DEFAULT 'verified'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
DEFINE FIELD created_at ON app_spec TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON app_spec TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_app_spec_project_key ON app_spec FIELDS project, entry_key UNIQUE;

-- Entity (graph nodes)
DEFINE TABLE entity SCHEMAFULL;
DEFINE FIELD entity_type ON entity TYPE string;
DEFINE FIELD name ON entity TYPE string;
-- FIX: [API-contract] 3.0 SCHEMAFULL: dynamic entity properties need
-- FLEXIBLE (same root as event.payload — 2.x silently dropped keys).
DEFINE FIELD OVERWRITE properties ON entity TYPE option<object> FLEXIBLE;
DEFINE FIELD content_hash ON entity TYPE option<string>;
DEFINE FIELD project ON entity TYPE option<record<project>>;
DEFINE FIELD created_at ON entity TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON entity TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_entity_type_name ON entity FIELDS entity_type, name UNIQUE;
DEFINE INDEX idx_entity_project ON entity FIELDS project;

-- Kanban is a status-lens over `roadmap` rows (entry_status), NOT a table.
-- The legacy `kanban` table was orphan: 0 rows in every project while the
-- real 94 open items live in `roadmap` (see kanban.rs:2,126). The table DDL
-- is removed here to end the split-brain. The empty table is left physically
-- present on existing stores (no REMOVE TABLE — destructive on production);
-- with no DDL re-creating it and no writer, it stays inert.

-- Events
DEFINE TABLE event SCHEMAFULL;
DEFINE FIELD session ON event TYPE option<record<session>>;
DEFINE FIELD event_type ON event TYPE string;
DEFINE FIELD source ON event TYPE string DEFAULT 'kavach';
DEFINE FIELD project ON event TYPE option<record<project>>;
DEFINE FIELD actor_id ON event TYPE string DEFAULT 'system';
-- FIX: [API-contract] SurrealDB 3.0 SCHEMAFULL rejects undeclared nested
-- object keys (2.x silently dropped them). payload is dynamic per-event
-- JSON — FLEXIBLE allows arbitrary keys on a SCHEMAFULL table.
-- SOURCE: surrealdb.com/docs/surrealql/statements/define/field (FLEXIBLE)
DEFINE FIELD OVERWRITE payload ON event TYPE option<object> FLEXIBLE;
DEFINE FIELD created_at ON event TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_event_session ON event FIELDS session;
DEFINE INDEX idx_event_type ON event FIELDS event_type;
DEFINE INDEX idx_event_project ON event FIELDS project, event_type, created_at;

-- Bandit-log store (harness-rl Wave P2): the durable RLVR (x, a, p, r) tuple.
-- SCHEMALESS because `payload` is an opaque, content-addressed JSON blob the
-- OPE layer deserializes — the store keeps no typed view of it. Declared (not
-- left to CREATE auto-creation) so a read of a fresh, never-appended log returns
-- an empty set, not SurrealDB 3.0's "table does not exist" error.
-- SOURCE: github.com/surrealdb/surrealdb/issues/139 (3.0 SELECT-on-undefined errors)
DEFINE TABLE bandit_log SCHEMALESS;
DEFINE FIELD created_at ON bandit_log TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_bandit_log_created ON bandit_log FIELDS created_at;

-- Dynamic gate-config overlay (unit.dynamic-gate-config-plane P1): the DB layer
-- in the resolver chain DB > file > compiled-default. A row OVERRIDES a gate
-- constant at runtime; absence falls through to the file/compiled default
-- (fail-closed — a missing row never disables a gate). `project` is the slug
-- string ('*' = the global row) NOT a record link, so the global row needs no
-- project record and lookups are a plain string match. Value is discriminated by
-- `kind`: exactly one of the four value_* columns is populated, validated at the
-- write edge (illegal cross-kind shapes unrepresentable). SOURCE: layered config
-- precedence (12-factor + k8s admission-policy overlay).
DEFINE TABLE gate_config SCHEMAFULL;
DEFINE FIELD project ON gate_config TYPE string;
DEFINE FIELD gate_key ON gate_config TYPE string;
DEFINE FIELD kind ON gate_config TYPE string
    ASSERT $value IN ['threshold', 'pattern_list', 'enabled', 'severity', 'text'];
DEFINE FIELD value_num ON gate_config TYPE option<number>;
DEFINE FIELD value_bool ON gate_config TYPE option<bool>;
DEFINE FIELD value_list ON gate_config TYPE option<array<string>>;
DEFINE FIELD value_text ON gate_config TYPE option<string>;
DEFINE FIELD updated_at ON gate_config TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_gate_config_project_key ON gate_config FIELDS project, gate_key UNIQUE;

-- Project parts (sub-components within a project: backend, frontend, etc.)
DEFINE TABLE part SCHEMAFULL;
DEFINE FIELD project ON part TYPE record<project>;
DEFINE FIELD part_name ON part TYPE string;
DEFINE FIELD part_path ON part TYPE string;
DEFINE FIELD part_type ON part TYPE string
    ASSERT $value IN ['backend', 'frontend', 'database', 'mobile', 'infra', 'docs', 'shared', 'other'];
DEFINE FIELD stack ON part TYPE option<string>;
DEFINE FIELD description ON part TYPE option<string>;
DEFINE FIELD created_at ON part TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON part TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_part_project_name ON part FIELDS project, part_name UNIQUE;
DEFINE INDEX idx_part_path ON part FIELDS part_path;

-- Run records (execution history and status tracking)
DEFINE TABLE run SCHEMAFULL;
DEFINE FIELD project ON run TYPE option<record<project>>;
DEFINE FIELD entry_key ON run TYPE string;
DEFINE FIELD branch ON run TYPE option<string>;
DEFINE FIELD status ON run TYPE string;
DEFINE FIELD command ON run TYPE option<string>;
DEFINE FIELD pid ON run TYPE option<int>;
DEFINE FIELD started_at ON run TYPE option<string>;
DEFINE FIELD finished_at ON run TYPE option<string>;
DEFINE FIELD exit_code ON run TYPE option<int>;
DEFINE FIELD cost_usd ON run TYPE option<float>;
DEFINE FIELD created_at ON run TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_run_project ON run FIELDS project;
DEFINE INDEX idx_run_project_started ON run FIELDS project, started_at;

-- Graph edge tables (created dynamically via RELATE)
-- Example edges:
--   session->works_on->project
--   roadmap->contains->unit
--   unit->depends_on->unit
--   unit->modifies->file
--   decision->references->entity

-- =============================================================================
-- FEEDBACK FIELD (v3 migration inline)
-- =============================================================================
-- Purpose: Actionable truth for kanban management
-- Use cases:
--   1. Issues found → feedback describes what to fix
--   2. Future implementation → feedback describes planned work
--   3. Done but not verified → feedback describes verification steps
-- =============================================================================
DEFINE FIELD IF NOT EXISTS feedback ON decision TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON research TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON pattern TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON roadmap TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON app_spec TYPE option<string>;
DEFINE FIELD IF NOT EXISTS access_count ON app_spec TYPE int DEFAULT 0;
DEFINE INDEX IF NOT EXISTS idx_roadmap_feedback ON roadmap FIELDS project, feedback;

-- Lower number = higher rank (1 picked before 2). NULL sorts AFTER explicit
-- values via NULLS LAST in read.rs. Existing rows materialize as NONE.
DEFINE FIELD IF NOT EXISTS priority ON roadmap TYPE option<int>;
DEFINE FIELD IF NOT EXISTS priority ON decision TYPE option<int>;
DEFINE INDEX IF NOT EXISTS idx_roadmap_priority ON roadmap FIELDS project, priority;

-- Lane-affinity sharding: a card may be pinned to a dispatch LANE so a session
-- running `KAVACH_LANE=<name>` runs its own prioritized slice, falls back to
-- the unlaned (NULL) general backlog when its lane drains, and never reaches a
-- foreign lane. NULL = unlaned. Roadmap only; indexed for the two-pass dispatch.
DEFINE FIELD IF NOT EXISTS lane ON roadmap TYPE option<string>;
DEFINE INDEX IF NOT EXISTS idx_roadmap_lane ON roadmap FIELDS project, lane;

-- Autonomous harness loop: a roadmap card may carry the dynamic-workflow
-- pattern the AI chose for it (`harness`, e.g. "worker-critic") and the path to
-- the compiled `workflow.js` the stop gate dispatches. NULL = no harness (the
-- card is handled by the ordinary kanban dispatch). See decision.goal-harness-6-patterns.
DEFINE FIELD IF NOT EXISTS harness ON roadmap TYPE option<string>;
DEFINE FIELD IF NOT EXISTS workflow_path ON roadmap TYPE option<string>;
-- Index the (project, harness) pair so L3's stop-gate dispatch can find the
-- harness-bearing cards for a project without a full-table scan.
DEFINE INDEX IF NOT EXISTS idx_roadmap_harness ON roadmap FIELDS project, harness;

-- owner-gate / block machinery REMOVED (owner directive 2026-06-16): a card is
-- either runnable or DELETED — never gate-flagged, never block-parked. The
-- `owner_gated` field + its index are dropped below for existing stores.
REMOVE INDEX IF EXISTS idx_roadmap_owner_gated ON roadmap;
REMOVE FIELD IF EXISTS owner_gated ON roadmap;
-- REMOVE FIELD drops the DEFINITION but NOT the bytes already stored per-row;
-- on a SCHEMAFULL table the orphan value then fails every subsequent UPDATE
-- ("Found field 'owner_gated', but no such field exists"). The companion
-- data-migration scrubs the stored value from existing rows. SCHEMAFULL order
-- is mandatory: REMOVE FIELD first (above), then UNSET (here). Idempotent: once
-- scrubbed the field is absent and UNSET is a no-op.
-- SOURCE: https://surrealdb.com/docs/surrealdb/surrealql/statements/update (UNSET)
--         https://github.com/orgs/surrealdb/discussions/191 (REMOVE FIELD then UNSET)
UPDATE roadmap UNSET owner_gated;

-- =============================================================================
-- Migration backfill: v2->v3 import did not materialize the `category` column
-- (v2 encoded it via table name only). SCHEMAFULL `category TYPE string` then
-- rejects any UPDATE/UPSERT of a legacy row ("Expected string but found NONE").
-- `category` ≡ table name by invariant. Idempotent: after first run the
-- WHERE matches nothing. Self-healing on future writes is handled by the
-- `VALUE $value OR '<table>'` clause on each category field above.
-- =============================================================================
UPDATE decision SET category = 'decision' WHERE category = NONE;
UPDATE research SET category = 'research' WHERE category = NONE;
UPDATE pattern SET category = 'pattern' WHERE category = NONE;
UPDATE roadmap SET category = 'roadmap' WHERE category = NONE;
UPDATE app_spec SET category = 'app_spec' WHERE category = NONE;

-- =============================================================================
-- session_runtime — durable harness runtime state, one row per session_id.
-- Distinct from the analytics-oriented `session` table above: this carries the
-- full SessionState. `state_blob` is the whole struct serialized as one string
-- (the existing INI text from to_ini_full()) so the schema stays stable as
-- SessionState grows (~50 fields and counting) — SCHEMAFULL would reject every
-- new undeclared field otherwise. Keyed by session_id so a /clear (new
-- session_id) cannot rehydrate a prior conversation's state. IF NOT EXISTS:
-- idempotent on already-running stores.
-- =============================================================================
DEFINE TABLE IF NOT EXISTS session_runtime SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS session_id ON session_runtime TYPE string;
DEFINE FIELD IF NOT EXISTS workdir ON session_runtime TYPE string;
DEFINE FIELD IF NOT EXISTS state_blob ON session_runtime TYPE string;
DEFINE FIELD IF NOT EXISTS updated_at ON session_runtime TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_session_runtime_sid ON session_runtime FIELDS session_id UNIQUE;

-- =============================================================================
-- L0 CONCEPT TIER — global cross-project knowledge graph nodes.
-- Concepts ride on the existing `entity` table with entity_type='concept' and
-- project=NONE. No new table; no migration. See plan: concept-graph-l0-tier.
-- =============================================================================
DEFINE INDEX IF NOT EXISTS idx_entity_kind ON entity FIELDS entity_type;
DEFINE ANALYZER IF NOT EXISTS concept_analyzer
    TOKENIZERS class FILTERS lowercase, snowball(english);
DEFINE INDEX IF NOT EXISTS idx_concept_fts
    ON TABLE entity COLUMNS properties.description
    FULLTEXT ANALYZER concept_analyzer BM25;
DEFINE FIELD IF NOT EXISTS embedding ON entity TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS idx_entity_embedding ON entity
    FIELDS embedding HNSW DIMENSION 384 DIST COSINE TYPE F32;

-- =============================================================================
-- bulk_manifest — single-RCA-bound batch edit authority for mechanical sweeps.
-- ONE manifest binds N edits sharing identical root_cause + fix_strategy.
-- Per-edit, the gate verifies {sweep_id env matches, file matches scope_glob,
-- diff matches fix_strategy lint_class} instead of re-asking for [RCA].
-- TTL'd to prevent pilot-exemption decay. Audit trail = manifest row +
-- bulk_apply events tagged with sweep_id. See: roadmap.unit.kavach-bulk-mode.
-- =============================================================================
DEFINE TABLE IF NOT EXISTS bulk_manifest SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS sweep_id ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS project ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS root_rca ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS scope_glob ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS lint_class ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS fix_strategy ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS blast_estimate ON bulk_manifest TYPE int;
DEFINE FIELD IF NOT EXISTS signed_by_session ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS approved_by ON bulk_manifest TYPE string;
DEFINE FIELD IF NOT EXISTS approved_at ON bulk_manifest TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS expires_at ON bulk_manifest TYPE datetime;
DEFINE FIELD IF NOT EXISTS conformance_applied ON bulk_manifest TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS conformance_refused ON bulk_manifest TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS conformance_drifted ON bulk_manifest TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS status ON bulk_manifest TYPE string DEFAULT "active";
DEFINE FIELD IF NOT EXISTS closed_at ON bulk_manifest TYPE option<datetime>;
DEFINE INDEX IF NOT EXISTS idx_bulk_manifest_sweep ON bulk_manifest FIELDS sweep_id UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_bulk_manifest_status ON bulk_manifest FIELDS status;
"#;
