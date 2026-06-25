//! Core tables: project + session. SOURCE: surrealdb.com/docs/surrealql/statements/define/field
pub(super) const DDL: &str = r#"
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
"#;
