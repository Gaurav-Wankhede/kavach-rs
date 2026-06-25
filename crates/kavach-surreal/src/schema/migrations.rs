//! Idempotent inline migrations: feedback/priority/lane/harness fields, owner-scrub, category backfill, session_runtime, concept tier, FTS indexes, bulk_manifest, nlm_doc.
pub(super) const DDL: &str = r"
-- FEEDBACK FIELD (v3 migration inline): actionable truth for kanban management.
DEFINE FIELD IF NOT EXISTS feedback ON decision TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON research TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON pattern TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON roadmap TYPE option<string>;
DEFINE FIELD IF NOT EXISTS feedback ON app_spec TYPE option<string>;
DEFINE FIELD IF NOT EXISTS access_count ON app_spec TYPE int DEFAULT 0;
DEFINE INDEX IF NOT EXISTS idx_roadmap_feedback ON roadmap FIELDS project, feedback;

-- priority: lower = higher rank (1 before 2). NULL sorts AFTER via NULLS LAST in read.rs.
DEFINE FIELD IF NOT EXISTS priority ON roadmap TYPE option<int>;
DEFINE FIELD IF NOT EXISTS priority ON decision TYPE option<int>;
DEFINE INDEX IF NOT EXISTS idx_roadmap_priority ON roadmap FIELDS project, priority;

-- lane-affinity sharding: a card pinned to a dispatch LANE; NULL = unlaned general backlog.
DEFINE FIELD IF NOT EXISTS lane ON roadmap TYPE option<string>;
DEFINE INDEX IF NOT EXISTS idx_roadmap_lane ON roadmap FIELDS project, lane;

-- autonomous harness loop: a roadmap card may carry its dynamic-workflow pattern + compiled workflow path. See decision.goal-harness-6-patterns.
DEFINE FIELD IF NOT EXISTS harness ON roadmap TYPE option<string>;
DEFINE FIELD IF NOT EXISTS workflow_path ON roadmap TYPE option<string>;
DEFINE INDEX IF NOT EXISTS idx_roadmap_harness ON roadmap FIELDS project, harness;

-- legacy-store scrub: drop owner_gated field+index, then UNSET stored bytes (idempotent).
REMOVE INDEX IF EXISTS idx_roadmap_owner_gated ON roadmap;
REMOVE FIELD IF EXISTS owner_gated ON roadmap;
UPDATE roadmap UNSET owner_gated;

-- migration backfill v2->v3: materialize `category` for legacy rows (category ≡ table name). Idempotent.
UPDATE decision SET category = 'decision' WHERE category = NONE;
UPDATE research SET category = 'research' WHERE category = NONE;
UPDATE pattern SET category = 'pattern' WHERE category = NONE;
UPDATE roadmap SET category = 'roadmap' WHERE category = NONE;
UPDATE app_spec SET category = 'app_spec' WHERE category = NONE;

-- session_runtime: durable harness runtime state, one row per session_id. state_blob is the whole SessionState serialized so the schema stays stable as it grows. Keyed by session_id.
DEFINE TABLE IF NOT EXISTS session_runtime SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS session_id ON session_runtime TYPE string;
DEFINE FIELD IF NOT EXISTS workdir ON session_runtime TYPE string;
DEFINE FIELD IF NOT EXISTS state_blob ON session_runtime TYPE string;
DEFINE FIELD IF NOT EXISTS updated_at ON session_runtime TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_session_runtime_sid ON session_runtime FIELDS session_id UNIQUE;

-- L0 CONCEPT TIER: global cross-project KG nodes ride on entity (entity_type='concept', project=NONE). See plan: concept-graph-l0-tier.
DEFINE INDEX IF NOT EXISTS idx_entity_kind ON entity FIELDS entity_type;
DEFINE ANALYZER IF NOT EXISTS concept_analyzer
    TOKENIZERS class FILTERS lowercase, snowball(english);
DEFINE INDEX IF NOT EXISTS idx_concept_fts
    ON TABLE entity COLUMNS properties.description
    FULLTEXT ANALYZER concept_analyzer BM25;

-- BRAIN-OS Gap 1: BM25/FTS over the whole memory corpus. One FULLTEXT index per field; match `field @@ 'terms'`, rank `search::score(n)`. BM25(1.2, 0.75) canonical.
DEFINE INDEX IF NOT EXISTS idx_decision_title_fts
    ON TABLE decision FIELDS title FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_decision_content_fts
    ON TABLE decision FIELDS content FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_roadmap_title_fts
    ON TABLE roadmap FIELDS title FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_roadmap_content_fts
    ON TABLE roadmap FIELDS content FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_research_title_fts
    ON TABLE research FIELDS title FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_research_content_fts
    ON TABLE research FIELDS content FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_pattern_title_fts
    ON TABLE pattern FIELDS title FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_pattern_content_fts
    ON TABLE pattern FIELDS content FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_app_spec_title_fts
    ON TABLE app_spec FIELDS title FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_app_spec_content_fts
    ON TABLE app_spec FIELDS content FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);

-- bulk_manifest: single-RCA-bound batch edit authority for mechanical sweeps. ONE manifest binds N edits sharing root_cause + fix_strategy. See: roadmap.unit.kavach-bulk-mode.
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

-- nlm_doc: NanoLM live-fetched docs corpus, retrieved live by BM25 (vectorless). One row per (source_url, heading) chunk. See roadmap.unit.nlm.p1c.
DEFINE TABLE IF NOT EXISTS nlm_doc SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS source_url ON nlm_doc TYPE string;
DEFINE FIELD IF NOT EXISTS heading ON nlm_doc TYPE string;
DEFINE FIELD IF NOT EXISTS body ON nlm_doc TYPE string;
DEFINE FIELD IF NOT EXISTS captured_at ON nlm_doc TYPE string;
DEFINE FIELD IF NOT EXISTS updated_at ON nlm_doc TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS idx_nlm_doc_chunk ON nlm_doc FIELDS source_url, heading UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_nlm_doc_body_fts
    ON nlm_doc FIELDS body FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
"#;
