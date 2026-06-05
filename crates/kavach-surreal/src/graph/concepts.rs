// L0 global concept tier — knowledge-graph nodes shared across projects.
// Concepts ride on the existing `entity` table (entity_type='concept',
// project=NONE). Edge names flow through type::table($edge) — SurrealDB 3.0
// parameter binding eliminates SQL interpolation in production paths.
//
// SOURCE: https://surrealdb.com/docs/learn/data-models/graph/overview
// SOURCE: https://surrealdb.com/docs/surrealql/parameters
// SOURCE: https://github.com/surrealdb/surrealdb/issues/2806 (resolved in 3.0)
pub mod delete;
pub mod query;
pub mod relate;
pub mod upsert;

pub use delete::{delete_concept, delete_concepts_by_prefix};
pub use query::{find_concept, list_concepts, search_concepts_fts};
pub use relate::relate_concepts;
pub use upsert::upsert_concept;
