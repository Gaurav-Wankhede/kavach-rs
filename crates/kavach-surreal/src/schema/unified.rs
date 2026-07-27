//! Unified Knowledge Graph schema: 32 tables, 37 edge types, 12+ FTS indexes, HNSW vector indexes.
//! Single source of truth for both kavach-rs and WebFind microservices.
pub(super) const DDL: &str = r"
-- ============================================================
-- SECTION 1: CORE TABLES (2)
-- ============================================================

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

-- ============================================================
-- SECTION 2: MEMORY TABLES (6)
-- ============================================================

-- Decision (typed table, replaces memory_entries category='decision')
DEFINE TABLE decision SCHEMAFULL;
DEFINE FIELD project ON decision TYPE record<project>;
DEFINE FIELD entry_key ON decision TYPE string;
DEFINE FIELD OVERWRITE category ON decision TYPE string VALUE $value OR 'decision';
DEFINE FIELD title ON decision TYPE string;
DEFINE FIELD content ON decision TYPE string;
DEFINE FIELD status ON decision TYPE string DEFAULT 'active';
DEFINE FIELD OVERWRITE entry_status ON decision TYPE string DEFAULT 'verified'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
DEFINE FIELD tags ON decision TYPE option<array<string>>;
DEFINE FIELD decay_score ON decision TYPE option<float>;
DEFINE FIELD access_count ON decision TYPE int DEFAULT 0;
DEFINE FIELD created_at ON decision TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON decision TYPE datetime DEFAULT time::now();
DEFINE FIELD accessed_at ON decision TYPE option<datetime>;
DEFINE FIELD feedback ON decision TYPE option<string>;
DEFINE FIELD priority ON decision TYPE option<int>;
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
DEFINE FIELD OVERWRITE entry_status ON research TYPE string DEFAULT 'verified'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
-- Semantic topic tags for content-aware search and cross-table joins with url_node
DEFINE FIELD topic_tags ON research TYPE option<array<string>>;
-- Evidence and confidence (AI-agent-critical fields)
DEFINE FIELD confidence_score ON research TYPE option<float>;
DEFINE FIELD source_count ON research TYPE int DEFAULT 0;
DEFINE FIELD consensus_level ON research TYPE option<string>;
DEFINE FIELD language ON research TYPE option<string>;
DEFINE FIELD domain ON research TYPE option<string>;
DEFINE FIELD decay_score ON research TYPE option<float>;
DEFINE FIELD access_count ON research TYPE int DEFAULT 0;
DEFINE FIELD created_at ON research TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON research TYPE datetime DEFAULT time::now();
DEFINE FIELD feedback ON research TYPE option<string>;
DEFINE INDEX idx_research_project_key ON research FIELDS project, entry_key UNIQUE;
DEFINE INDEX idx_research_topic_tags ON research FIELDS topic_tags;
DEFINE INDEX idx_research_language ON research FIELDS language;
DEFINE INDEX idx_research_domain ON research FIELDS domain;
DEFINE INDEX idx_research_consensus ON research FIELDS consensus_level;

-- Pattern (typed table, replaces memory_entries category='pattern')
DEFINE TABLE pattern SCHEMAFULL;
DEFINE FIELD project ON pattern TYPE record<project>;
DEFINE FIELD entry_key ON pattern TYPE string;
DEFINE FIELD OVERWRITE category ON pattern TYPE string VALUE $value OR 'pattern';
DEFINE FIELD title ON pattern TYPE string;
DEFINE FIELD content ON pattern TYPE string;
DEFINE FIELD status ON pattern TYPE string DEFAULT 'active';
DEFINE FIELD OVERWRITE entry_status ON pattern TYPE string DEFAULT 'verified'
    ASSERT $value IN ['todo', 'in_progress', 'done', 'verified'];
DEFINE FIELD decay_score ON pattern TYPE option<float>;
DEFINE FIELD access_count ON pattern TYPE int DEFAULT 0;
DEFINE FIELD created_at ON pattern TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON pattern TYPE datetime DEFAULT time::now();
DEFINE FIELD feedback ON pattern TYPE option<string>;
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
DEFINE FIELD feedback ON roadmap TYPE option<string>;
DEFINE FIELD priority ON roadmap TYPE option<int>;
DEFINE FIELD exec_prompt ON roadmap TYPE option<string>;
DEFINE FIELD lane ON roadmap TYPE option<string>;
DEFINE FIELD harness ON roadmap TYPE option<string>;
DEFINE FIELD workflow_path ON roadmap TYPE option<string>;
DEFINE INDEX idx_roadmap_project_key ON roadmap FIELDS project, entry_key UNIQUE;
DEFINE INDEX idx_roadmap_status ON roadmap FIELDS project, entry_status;
DEFINE INDEX idx_roadmap_feedback ON roadmap FIELDS project, feedback;
DEFINE INDEX idx_roadmap_priority ON roadmap FIELDS project, priority;
DEFINE INDEX idx_roadmap_lane ON roadmap FIELDS project, lane;
DEFINE INDEX idx_roadmap_harness ON roadmap FIELDS project, harness;

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
DEFINE FIELD access_count ON app_spec TYPE int DEFAULT 0;
DEFINE FIELD created_at ON app_spec TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON app_spec TYPE datetime DEFAULT time::now();
DEFINE FIELD feedback ON app_spec TYPE option<string>;
DEFINE INDEX idx_app_spec_project_key ON app_spec FIELDS project, entry_key UNIQUE;

-- Citation (official-docs awareness; hybrid table+graph DAG root)
DEFINE TABLE citation SCHEMAFULL;
DEFINE FIELD project ON citation TYPE record<project>;
DEFINE FIELD entry_key ON citation TYPE string;
DEFINE FIELD OVERWRITE category ON citation TYPE string VALUE $value OR 'citation';
DEFINE FIELD name ON citation TYPE string;
DEFINE FIELD metadata ON citation TYPE array<object> DEFAULT [];
DEFINE FIELD metadata.*.slug ON citation TYPE string;
DEFINE FIELD metadata.*.desc ON citation TYPE string DEFAULT '';
DEFINE FIELD metadata.*.url ON citation TYPE string
    ASSERT string::len($value) > 0;
DEFINE FIELD metadata.*.parent ON citation TYPE option<string>;
DEFINE FIELD metadata.*.depends_on ON citation TYPE option<string>;
DEFINE FIELD metadata.*.best_practice ON citation TYPE string DEFAULT '';
DEFINE FIELD metadata.*.worst_practice ON citation TYPE string DEFAULT '';
DEFINE FIELD metadata.*.tradeoff ON citation TYPE string DEFAULT '';
DEFINE FIELD metadata.*.created_at ON citation TYPE datetime VALUE $value OR time::now();
DEFINE FIELD metadata.*.updated_at ON citation TYPE datetime VALUE time::now();
DEFINE FIELD access_count ON citation TYPE int DEFAULT 0;
DEFINE FIELD created_at ON citation TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON citation TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_citation_project_key ON citation FIELDS project, entry_key UNIQUE;

-- ============================================================
-- SECTION 3: GRAPH/OPERATIONAL TABLES (8)
-- ============================================================

-- Entity (graph nodes: concepts, anti-patterns, deployed-policies, etc.)
DEFINE TABLE entity SCHEMAFULL;
DEFINE FIELD entity_type ON entity TYPE string;
DEFINE FIELD name ON entity TYPE string;
DEFINE FIELD OVERWRITE properties ON entity TYPE option<object> FLEXIBLE;
DEFINE FIELD content_hash ON entity TYPE option<string>;
DEFINE FIELD project ON entity TYPE option<record<project>>;
-- Authority and provenance (AI-agent-critical fields)
DEFINE FIELD authority_score ON entity TYPE option<float>;
DEFINE FIELD source_url ON entity TYPE option<string>;
DEFINE FIELD created_at ON entity TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON entity TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_entity_type_name ON entity FIELDS entity_type, name UNIQUE;
DEFINE INDEX idx_entity_project ON entity FIELDS project;
DEFINE INDEX idx_entity_kind ON entity FIELDS entity_type;
DEFINE INDEX idx_entity_authority ON entity FIELDS authority_score;

-- Events (audit trail)
DEFINE TABLE event SCHEMAFULL;
DEFINE FIELD session ON event TYPE option<record<session>>;
DEFINE FIELD event_type ON event TYPE string;
DEFINE FIELD source ON event TYPE string DEFAULT 'kavach';
DEFINE FIELD project ON event TYPE option<record<project>>;
DEFINE FIELD actor_id ON event TYPE string DEFAULT 'system';
DEFINE FIELD OVERWRITE payload ON event TYPE option<object> FLEXIBLE;
DEFINE FIELD created_at ON event TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_event_session ON event FIELDS session;
DEFINE INDEX idx_event_type ON event FIELDS event_type;
DEFINE INDEX idx_event_project ON event FIELDS project, event_type, created_at;

-- Bandit-log (RLVR tuples, SCHEMALESS opaque blob)
DEFINE TABLE bandit_log SCHEMALESS;
DEFINE FIELD created_at ON bandit_log TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_bandit_log_created ON bandit_log FIELDS created_at;

-- Gate-config overlay (DB > file > compiled-default)
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

-- Project parts (sub-components)
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

-- Run records (execution history)
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

-- Algo-decision (algorithm choice records)
DEFINE TABLE algo_decision SCHEMAFULL;
DEFINE FIELD project ON algo_decision TYPE record<project>;
DEFINE FIELD problem_class ON algo_decision TYPE string;
DEFINE FIELD chosen ON algo_decision TYPE string;
DEFINE FIELD time_complexity ON algo_decision TYPE string;
DEFINE FIELD space_complexity ON algo_decision TYPE string;
DEFINE FIELD file_path ON algo_decision TYPE string;
DEFINE FIELD search_year ON algo_decision TYPE int;
DEFINE FIELD search_month ON algo_decision TYPE int;
DEFINE FIELD created_at ON algo_decision TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_algo_unique ON algo_decision FIELDS project, problem_class, file_path UNIQUE;
DEFINE INDEX idx_algo_project ON algo_decision FIELDS project, created_at;

-- Arch-decision (architecture choice records)
DEFINE TABLE arch_decision SCHEMAFULL;
DEFINE FIELD project ON arch_decision TYPE record<project>;
DEFINE FIELD pattern ON arch_decision TYPE string;
DEFINE FIELD scope ON arch_decision TYPE string;
DEFINE FIELD cap_choice ON arch_decision TYPE option<string>;
DEFINE FIELD failure_mode ON arch_decision TYPE string;
DEFINE FIELD tradeoff ON arch_decision TYPE string;
DEFINE FIELD file_path ON arch_decision TYPE string;
DEFINE FIELD search_year ON arch_decision TYPE int;
DEFINE FIELD search_month ON arch_decision TYPE int;
DEFINE FIELD created_at ON arch_decision TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_arch_unique ON arch_decision FIELDS project, pattern, file_path UNIQUE;
DEFINE INDEX idx_arch_project ON arch_decision FIELDS project, created_at;

-- ============================================================
-- SECTION 4: ENGINE TABLES (3)
-- ============================================================

-- Gate-pattern (self-evolved false-positive fixes)
DEFINE TABLE gate_pattern SCHEMAFULL;
DEFINE FIELD project ON gate_pattern TYPE record<project>;
DEFINE FIELD tool_name ON gate_pattern TYPE string;
DEFINE FIELD gate_name ON gate_pattern TYPE string;
DEFINE FIELD error_tokens ON gate_pattern TYPE string;
DEFINE FIELD fix_strategy ON gate_pattern TYPE string;
DEFINE FIELD imperative_rewrite ON gate_pattern TYPE string;
DEFINE FIELD dsa_rationale ON gate_pattern TYPE string;
DEFINE FIELD occurrence_count ON gate_pattern TYPE int DEFAULT 1;
DEFINE FIELD bloom_bytes ON gate_pattern TYPE option<bytes>;
DEFINE FIELD tier ON gate_pattern TYPE string DEFAULT 'research'
    ASSERT $value IN ['research', 'autonomous'];
DEFINE FIELD created_at ON gate_pattern TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON gate_pattern TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_gate_pattern_project_tier ON gate_pattern FIELDS project, tier, occurrence_count;
DEFINE INDEX idx_gate_pattern_tool ON gate_pattern FIELDS tool_name, gate_name;

-- RAG-tree (tokenized source trees, SCHEMALESS)
DEFINE TABLE rag_tree SCHEMALESS;
DEFINE FIELD source ON rag_tree TYPE string;
DEFINE FIELD built_at ON rag_tree TYPE datetime DEFAULT time::now();
DEFINE FIELD tree_json ON rag_tree TYPE bytes;
DEFINE FIELD source_hash ON rag_tree TYPE string;
DEFINE FIELD source_dir ON rag_tree TYPE string DEFAULT '';
DEFINE FIELD created_at ON rag_tree TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON rag_tree TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_rag_tree_source ON rag_tree FIELDS source UNIQUE;

-- Procedure (extracted workflows from trajectories)
DEFINE TABLE procedure SCHEMAFULL;
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
DEFINE INDEX idx_procedure_name ON procedure FIELDS name UNIQUE;
DEFINE INDEX idx_procedure_trigger ON procedure FIELDS trigger_pattern;
DEFINE INDEX idx_procedure_reliability ON procedure FIELDS reliability_score;

-- ============================================================
-- SECTION 5: MEMORY V2 TABLES (6)
-- ============================================================

-- Trajectory (execution paths)
DEFINE TABLE trajectory SCHEMAFULL;
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
DEFINE INDEX idx_trajectory_project ON trajectory FIELDS project;
DEFINE INDEX idx_trajectory_outcome ON trajectory FIELDS project, outcome;
DEFINE INDEX idx_trajectory_session ON trajectory FIELDS session_id;

-- Working-memory (short-term TTL context)
DEFINE TABLE working_memory SCHEMAFULL;
DEFINE FIELD project ON working_memory TYPE record<project>;
DEFINE FIELD session_id ON working_memory TYPE string;
DEFINE FIELD key ON working_memory TYPE string;
DEFINE FIELD value ON working_memory TYPE object;
DEFINE FIELD ttl_seconds ON working_memory TYPE int DEFAULT 3600;
DEFINE FIELD created_at ON working_memory TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_working_memory_session ON working_memory FIELDS session_id, key UNIQUE;

-- Episode (compressed session summaries)
DEFINE TABLE episode SCHEMAFULL;
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
DEFINE INDEX idx_episode_project ON episode FIELDS project;
DEFINE INDEX idx_episode_session ON episode FIELDS session_id UNIQUE;

-- Context-snapshot (pre-computed aggregates)
DEFINE TABLE context_snapshot SCHEMAFULL;
DEFINE FIELD project ON context_snapshot TYPE record<project>;
DEFINE FIELD kanban_counts ON context_snapshot TYPE object;
DEFINE FIELD active_items ON context_snapshot TYPE array<object> DEFAULT [];
DEFINE FIELD recent_decisions ON context_snapshot TYPE array<object> DEFAULT [];
DEFINE FIELD hot_procedures ON context_snapshot TYPE array<object> DEFAULT [];
DEFINE FIELD computed_at ON context_snapshot TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_context_snapshot_project ON context_snapshot FIELDS project UNIQUE;

-- Roadmap-version (temporal journal for roadmap changes)
DEFINE TABLE roadmap_version SCHEMAFULL;
DEFINE FIELD roadmap ON roadmap_version TYPE record<roadmap>;
DEFINE FIELD version ON roadmap_version TYPE int;
DEFINE FIELD entry_status ON roadmap_version TYPE string;
DEFINE FIELD title ON roadmap_version TYPE string;
DEFINE FIELD content ON roadmap_version TYPE string;
DEFINE FIELD changed_by ON roadmap_version TYPE string DEFAULT 'agent';
DEFINE FIELD change_reason ON roadmap_version TYPE option<string>;
DEFINE FIELD created_at ON roadmap_version TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_roadmap_version_roadmap ON roadmap_version FIELDS roadmap;
DEFINE INDEX idx_roadmap_version_num ON roadmap_version FIELDS roadmap, version;

-- Decision-version (temporal journal for decision changes)
DEFINE TABLE decision_version SCHEMAFULL;
DEFINE FIELD decision ON decision_version TYPE record<decision>;
DEFINE FIELD version ON decision_version TYPE int;
DEFINE FIELD status ON decision_version TYPE string;
DEFINE FIELD title ON decision_version TYPE string;
DEFINE FIELD content ON decision_version TYPE string;
DEFINE FIELD changed_by ON decision_version TYPE string DEFAULT 'agent';
DEFINE FIELD change_reason ON decision_version TYPE option<string>;
DEFINE FIELD created_at ON decision_version TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_decision_version_decision ON decision_version FIELDS decision;

-- ============================================================
-- SECTION 6: INFRASTRUCTURE TABLES (3)
-- ============================================================

-- Session-runtime (durable harness state)
DEFINE TABLE session_runtime SCHEMAFULL;
DEFINE FIELD session_id ON session_runtime TYPE string;
DEFINE FIELD workdir ON session_runtime TYPE string;
DEFINE FIELD state_blob ON session_runtime TYPE string;
DEFINE FIELD updated_at ON session_runtime TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_session_runtime_sid ON session_runtime FIELDS session_id UNIQUE;

-- Bulk-manifest (batch edit authority)
DEFINE TABLE bulk_manifest SCHEMAFULL;
DEFINE FIELD sweep_id ON bulk_manifest TYPE string;
DEFINE FIELD project ON bulk_manifest TYPE string;
DEFINE FIELD root_rca ON bulk_manifest TYPE string;
DEFINE FIELD scope_glob ON bulk_manifest TYPE string;
DEFINE FIELD lint_class ON bulk_manifest TYPE string;
DEFINE FIELD fix_strategy ON bulk_manifest TYPE string;
DEFINE FIELD blast_estimate ON bulk_manifest TYPE int;
DEFINE FIELD signed_by_session ON bulk_manifest TYPE string;
DEFINE FIELD approved_by ON bulk_manifest TYPE string;
DEFINE FIELD approved_at ON bulk_manifest TYPE datetime DEFAULT time::now();
DEFINE FIELD expires_at ON bulk_manifest TYPE datetime;
DEFINE FIELD conformance_applied ON bulk_manifest TYPE int DEFAULT 0;
DEFINE FIELD conformance_refused ON bulk_manifest TYPE int DEFAULT 0;
DEFINE FIELD conformance_drifted ON bulk_manifest TYPE int DEFAULT 0;
DEFINE FIELD status ON bulk_manifest TYPE string DEFAULT 'active';
DEFINE FIELD closed_at ON bulk_manifest TYPE option<datetime>;
DEFINE INDEX idx_bulk_manifest_sweep ON bulk_manifest FIELDS sweep_id UNIQUE;
DEFINE INDEX idx_bulk_manifest_status ON bulk_manifest FIELDS status;

-- NLM-doc (NanoLM live-fetched docs corpus)
DEFINE TABLE nlm_doc SCHEMAFULL;
DEFINE FIELD source_url ON nlm_doc TYPE string;
DEFINE FIELD heading ON nlm_doc TYPE string;
DEFINE FIELD body ON nlm_doc TYPE string;
DEFINE FIELD captured_at ON nlm_doc TYPE string;
DEFINE FIELD updated_at ON nlm_doc TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_nlm_doc_chunk ON nlm_doc FIELDS source_url, heading UNIQUE;

-- ============================================================
-- SECTION 7: WEBFIND TABLES (5)
-- ============================================================

-- URL-node (crawled URLs, nodes in the link graph — AI-agent-optimized)
DEFINE TABLE url_node SCHEMAFULL;
-- Identity
DEFINE FIELD url ON url_node TYPE string ASSERT $value != NONE;
DEFINE FIELD domain ON url_node TYPE string;
DEFINE FIELD source ON url_node TYPE string;
DEFINE FIELD depth ON url_node TYPE number DEFAULT 0;
DEFINE FIELD priority ON url_node TYPE number;
DEFINE FIELD lastmod ON url_node TYPE option<datetime>;
DEFINE FIELD changefreq ON url_node TYPE option<string>;
DEFINE FIELD discovered_at ON url_node TYPE datetime;
DEFINE FIELD crawled ON url_node TYPE bool DEFAULT false;
-- Content classification
DEFINE FIELD content_type ON url_node TYPE option<string>;
DEFINE FIELD topic_tags ON url_node TYPE option<array<string>>;
-- Content metadata (AI-agent-critical fields)
DEFINE FIELD title ON url_node TYPE option<string>;
DEFINE FIELD description ON url_node TYPE option<string>;
DEFINE FIELD author ON url_node TYPE option<string>;
DEFINE FIELD language ON url_node TYPE string DEFAULT 'en';
DEFINE FIELD published_at ON url_node TYPE option<datetime>;
DEFINE FIELD modified_at ON url_node TYPE option<datetime>;
DEFINE FIELD word_count ON url_node TYPE option<int>;
DEFINE FIELD reading_time_seconds ON url_node TYPE option<int>;
DEFINE FIELD reading_level ON url_node TYPE option<string>;
-- Trust signals
DEFINE FIELD ssl_valid ON url_node TYPE bool DEFAULT true;
DEFINE FIELD is_paywalled ON url_node TYPE bool DEFAULT false;
DEFINE FIELD confidence_score ON url_node TYPE option<float>;
DEFINE FIELD domain_authority ON url_node TYPE option<float>;
-- UI display
DEFINE FIELD favicon_url ON url_node TYPE option<string>;
DEFINE FIELD thumbnail_url ON url_node TYPE option<string>;
-- Dedup and refresh
DEFINE FIELD content_hash ON url_node TYPE option<string>;
DEFINE FIELD refresh_interval ON url_node TYPE option<string>;
-- Vector embedding for semantic search (generated by WebFind)
DEFINE FIELD embedding ON url_node TYPE option<array<float>>;
-- Indexes
DEFINE INDEX idx_url_node_url ON url_node FIELDS url UNIQUE;
DEFINE INDEX idx_url_node_domain ON url_node FIELDS domain;
DEFINE INDEX idx_url_node_content_type ON url_node FIELDS content_type;
DEFINE INDEX idx_url_node_topic_tags ON url_node FIELDS topic_tags;
DEFINE INDEX idx_url_node_language ON url_node FIELDS language;
DEFINE INDEX idx_url_node_published ON url_node FIELDS published_at;
DEFINE INDEX idx_url_node_modified ON url_node FIELDS modified_at;
DEFINE INDEX idx_url_node_author ON url_node FIELDS author;
DEFINE INDEX idx_url_node_content_hash ON url_node FIELDS content_hash;
DEFINE INDEX idx_url_node_domain_authority ON url_node FIELDS domain_authority;
-- HNSW vector index for semantic URL search
DEFINE INDEX idx_url_node_embedding ON url_node FIELDS embedding
    HNSW DIMENSION 384 DIST COSINE;

-- Link-edge (directed hyperlinks between URLs)
DEFINE TABLE link_edge SCHEMAFULL TYPE RELATION;
DEFINE FIELD in ON link_edge TYPE record<url_node>;
DEFINE FIELD out ON link_edge TYPE record<url_node>;
DEFINE FIELD anchor_text ON link_edge TYPE option<string>;

-- Graph-meta (version counter for cache invalidation)
DEFINE TABLE graph_meta SCHEMAFULL;
DEFINE FIELD value ON graph_meta TYPE number DEFAULT 0;

-- IP-health (fingerprint IP health tracking)
DEFINE TABLE ip_health SCHEMAFULL;
DEFINE FIELD ip ON ip_health TYPE string ASSERT $value != NONE;
DEFINE FIELD fingerprint_id ON ip_health TYPE string;
DEFINE FIELD isp ON ip_health TYPE string;
DEFINE FIELD asn ON ip_health TYPE string;
DEFINE FIELD country ON ip_health TYPE string;
DEFINE FIELD user_agent ON ip_health TYPE string;
DEFINE FIELD accept_language ON ip_health TYPE string;
DEFINE FIELD device_class ON ip_health TYPE string;
DEFINE FIELD geo_region ON ip_health TYPE string;
DEFINE FIELD working ON ip_health TYPE bool DEFAULT true;
DEFINE FIELD success_count ON ip_health TYPE number DEFAULT 0;
DEFINE FIELD failure_count ON ip_health TYPE number DEFAULT 0;
DEFINE FIELD last_used ON ip_health TYPE option<datetime>;
DEFINE FIELD last_error ON ip_health TYPE option<string>;
DEFINE FIELD discarded_at ON ip_health TYPE option<datetime>;
DEFINE INDEX idx_ip_health_ip ON ip_health FIELDS ip UNIQUE;

-- Fingerprint-log (append-only audit trail)
DEFINE TABLE fingerprint_log SCHEMAFULL;
DEFINE FIELD fingerprint_id ON fingerprint_log TYPE string;
DEFINE FIELD ip ON fingerprint_log TYPE string;
DEFINE FIELD isp ON fingerprint_log TYPE string;
DEFINE FIELD asn ON fingerprint_log TYPE string;
DEFINE FIELD country ON fingerprint_log TYPE string;
DEFINE FIELD user_agent ON fingerprint_log TYPE string;
DEFINE FIELD accept_language ON fingerprint_log TYPE string;
DEFINE FIELD device_class ON fingerprint_log TYPE string;
DEFINE FIELD geo_region ON fingerprint_log TYPE string;
DEFINE FIELD status_code ON fingerprint_log TYPE option<number>;
DEFINE FIELD error ON fingerprint_log TYPE option<string>;
DEFINE FIELD used_at ON fingerprint_log TYPE datetime;

-- ============================================================
-- SECTION 8: BRIDGE RELATIONS (kavach <-> WebFind)
-- ============================================================

-- Research-to-URL bridge: tracks which web pages sourced a research finding.
-- Created via: RELATE research:<id> -> sourced_from -> url_node:<id>
DEFINE TABLE sourced_from SCHEMAFULL TYPE RELATION;
DEFINE FIELD in ON sourced_from TYPE record<research>;
DEFINE FIELD out ON sourced_from TYPE record<url_node>;
DEFINE FIELD fetched_at ON sourced_from TYPE datetime DEFAULT time::now();

-- ============================================================
-- SECTION 9: FULL-TEXT SEARCH INDEXES (12+)
-- ============================================================

-- Analyzer: shared across all FTS indexes
DEFINE ANALYZER IF NOT EXISTS concept_analyzer
    TOKENIZERS class FILTERS lowercase, snowball(english);

-- Memory table FTS (BM25 scoring)
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

-- Entity FTS (concept graph search)
DEFINE INDEX IF NOT EXISTS idx_concept_fts
    ON TABLE entity COLUMNS properties.description
    FULLTEXT ANALYZER concept_analyzer BM25;

-- NLM-doc FTS
DEFINE INDEX IF NOT EXISTS idx_nlm_doc_body_fts
    ON nlm_doc FIELDS body FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);

-- WebFind FTS (search crawled content)
DEFINE INDEX IF NOT EXISTS idx_url_node_fts
    ON TABLE url_node FIELDS url FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_url_node_title_fts
    ON TABLE url_node FIELDS title FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);
DEFINE INDEX IF NOT EXISTS idx_url_node_desc_fts
    ON TABLE url_node FIELDS description FULLTEXT ANALYZER concept_analyzer BM25(1.2, 0.75);

-- ============================================================
-- SECTION 10: VECTOR EMBEDDING INDEXES (HNSW)
-- ============================================================

-- Research content embedding (semantic search over research findings)
DEFINE FIELD IF NOT EXISTS embedding ON research TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS idx_research_embedding ON research FIELDS embedding
    HNSW DIMENSION 384 DIST COSINE;

-- Entity description embedding (semantic search over concepts)
DEFINE FIELD IF NOT EXISTS embedding ON entity TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS idx_entity_embedding ON entity FIELDS embedding
    HNSW DIMENSION 384 DIST COSINE;

-- NLM-doc body embedding (semantic search over documentation)
DEFINE FIELD IF NOT EXISTS embedding ON nlm_doc TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS idx_nlm_doc_embedding ON nlm_doc FIELDS embedding
    HNSW DIMENSION 384 DIST COSINE;

-- Legacy store scrub (idempotent)
REMOVE INDEX IF EXISTS idx_roadmap_owner_gated ON roadmap;
REMOVE FIELD IF EXISTS owner_gated ON roadmap;
UPDATE roadmap UNSET owner_gated;
";
