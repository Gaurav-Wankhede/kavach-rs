//! Graph nodes (entity), events, and operational tables: gate_config, part, run, bandit_log.
pub(super) const DDL: &str = r"
-- Entity (graph nodes)
DEFINE TABLE entity SCHEMAFULL;
DEFINE FIELD entity_type ON entity TYPE string;
DEFINE FIELD name ON entity TYPE string;
-- 3.0 SCHEMAFULL: dynamic entity properties need FLEXIBLE (2.x dropped keys).
DEFINE FIELD OVERWRITE properties ON entity TYPE option<object> FLEXIBLE;
DEFINE FIELD content_hash ON entity TYPE option<string>;
DEFINE FIELD project ON entity TYPE option<record<project>>;
DEFINE FIELD created_at ON entity TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON entity TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_entity_type_name ON entity FIELDS entity_type, name UNIQUE;
DEFINE INDEX idx_entity_project ON entity FIELDS project;

-- Events
DEFINE TABLE event SCHEMAFULL;
DEFINE FIELD session ON event TYPE option<record<session>>;
DEFINE FIELD event_type ON event TYPE string;
DEFINE FIELD source ON event TYPE string DEFAULT 'kavach';
DEFINE FIELD project ON event TYPE option<record<project>>;
DEFINE FIELD actor_id ON event TYPE string DEFAULT 'system';
-- payload is dynamic per-event JSON; FLEXIBLE allows arbitrary keys on SCHEMAFULL. SOURCE: surrealdb.com/docs/surrealql/statements/define/field
DEFINE FIELD OVERWRITE payload ON event TYPE option<object> FLEXIBLE;
DEFINE FIELD created_at ON event TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_event_session ON event FIELDS session;
DEFINE INDEX idx_event_type ON event FIELDS event_type;
DEFINE INDEX idx_event_project ON event FIELDS project, event_type, created_at;

-- Bandit-log store (harness-rl Wave P2): durable RLVR (x, a, p, r) tuple. SCHEMALESS opaque blob; declared so a fresh read returns empty not a 3.0 missing-table error. SOURCE: github.com/surrealdb/surrealdb/issues/139
DEFINE TABLE bandit_log SCHEMALESS;
DEFINE FIELD created_at ON bandit_log TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_bandit_log_created ON bandit_log FIELDS created_at;

-- Dynamic gate-config overlay: DB layer in resolver chain DB > file > compiled-default. project is slug string ('*'=global), value discriminated by kind. SOURCE: 12-factor + k8s admission-policy overlay.
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
";
