//! Typed memory tables: decision/research/pattern/roadmap/app_spec/citation (replaces memory_entries categories).
pub(super) const DDL: &str = r"
-- Decision (typed table, replaces memory_entries category='decision')
DEFINE TABLE decision SCHEMAFULL;
DEFINE FIELD project ON decision TYPE record<project>;
DEFINE FIELD entry_key ON decision TYPE string;
-- 3.0 SCHEMAFULL rejects undeclared SET fields (2.x dropped them); declare category. SOURCE: surrealdb.com/docs DEFINE FIELD.
DEFINE FIELD OVERWRITE category ON decision TYPE string VALUE $value OR 'decision';
DEFINE FIELD title ON decision TYPE string;
DEFINE FIELD content ON decision TYPE string;
DEFINE FIELD status ON decision TYPE string DEFAULT 'active';
-- Knowledge rows are SETTLED facts, not work; default `verified` so they skip the todo census (was 'todo' -> 308 phantoms).
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
-- Research findings are cached SETTLED facts, not dispatchable work — default `verified`.
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
-- Gate patterns are LEARNED facts (false-positive fixes), not dispatchable work — default `verified`.
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

-- Citation (official-docs awareness; hybrid table+graph DAG root). DAG edges are RELATE relations (C2).
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
"#;
