//! Graph storage — `GraphStore` extension trait over `TursoStore`.
//!
//! Phase 3 parity with graph/dynamic.rs: SurrealDB kept entities in the
//! `entity` table and relationships as per-type edge tables created by RELATE.
//! libSQL models this as two plain tables (`entities`, `entity_edges`) with
//! recursive-CTE traversal. Edge identity is `(from_id, rel_type, to_id)`.

use std::fmt::Write as _;
use std::future::Future;

use super::{TursoError, TursoResult, TursoStore};

/// Entity types seen in the wild (fixture parity list — not a CHECK constraint;
/// dynamic.rs never constrained it either).
pub const ENTITY_TYPES: [&str; 8] = [
    "skill", "rule", "gate", "project", "file", "memory", "session", "concept",
];

/// Relation allowlist mirroring `ALLOWED_RELS` in graph/dynamic.rs.
pub const ALLOWED_RELS: &[&str] = &[
    // Workflow / project-DAG edges (original set)
    "contains",
    "depends_on",
    "modifies",
    "references",
    "mentions",
    "works_on",
    "owns",
    "uses_skill",
    "cross_invoke",
    "invoke",
    "uses_pattern",
    // Inter-entry semantic edges projected by upsert_relationships
    "supersedes",
    "blocks",
    "in_scope",
    "uses_algorithm",
    "session_uses_skill",
    "solves",
    // Ontology edges (L0 concept <-> L0 concept)
    "is_a",
    "part_of",
    "prerequisite_of",
    "alternative_to",
    "composes",
    "mitigates",
    "instance_of",
    "subsumes",
    // Bridge edges (L1 project_entity -> L0 concept)
    "implements",
    "discusses",
    "references_concept",
    "violates",
    // Mistake-tier edges
    "fired_gate",
    "triggered_in_session",
    "correct_action_ref",
    // Citation-tier edges
    "cite",
    "parent",
];

/// Validate a relation name against the allowlist (typo-guard).
///
/// # Errors
/// Returns [`TursoError::Mapping`] for unknown relations.
pub fn validate_rel(rel_type: &str) -> TursoResult<()> {
    if ALLOWED_RELS.contains(&rel_type) {
        Ok(())
    } else {
        Err(TursoError::Mapping(format!(
            "unknown relationship type: {rel_type}"
        )))
    }
}

/// Graph-owned DDL, executed by `connection::init_schema`.
pub const DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS entities (
              id TEXT PRIMARY KEY,
              entity_type TEXT NOT NULL,
              name TEXT NOT NULL,
              properties TEXT,
              content_hash TEXT,
              project TEXT
            )",
    "CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type)",
    "CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name)",
    "CREATE TABLE IF NOT EXISTS entity_edges (
              from_id TEXT NOT NULL,
              rel_type TEXT NOT NULL,
              to_id TEXT NOT NULL,
              weight REAL NOT NULL DEFAULT 1.0,
              created_at TEXT NOT NULL,
              PRIMARY KEY (from_id, rel_type, to_id)
            )",
    "CREATE INDEX IF NOT EXISTS idx_edges_to ON entity_edges(to_id)",
];

/// A graph entity row — parity of `graph::types::Entity` without RecordId.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct GraphEntity {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub properties: Option<String>,
    pub content_hash: Option<String>,
    pub project: Option<String>,
}

impl GraphEntity {
    /// Construct from plain parts (cross-crate struct literals are refused by
    /// E0639 because the struct is `#[non_exhaustive]`).
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        entity_type: impl Into<String>,
        name: impl Into<String>,
        properties: Option<String>,
        content_hash: Option<String>,
        project: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            entity_type: entity_type.into(),
            name: name.into(),
            properties,
            content_hash,
            project,
        }
    }
}

/// A directed edge — parity of `graph::dynamic::EdgeRow` plus weight.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct GraphEdge {
    pub from: String,
    pub rel_type: String,
    pub to: String,
    pub weight: f64,
}

/// One outgoing relation with its resolved target — parity of
/// `graph::dynamic::RelatedRow`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct RelatedRow {
    pub rel_type: String,
    pub weight: f64,
    pub target: GraphEntity,
}

/// One edge between two in-set nodes — parity of
/// `graph::dynamic::EdgeRow`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct EdgeRow {
    pub from: String,
    pub rel_type: String,
    pub to: String,
}

/// Entity/edge operations on the embedded store.
pub trait GraphStore {
    /// Insert or update an entity (`id` caller-supplied; ULID key space kept
    /// for migration parity).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn upsert_entity(
        &self,
        e: &GraphEntity,
    ) -> impl Future<Output = TursoResult<()>> + Send;

    /// Fetch one entity by id.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn get_entity(
        &self,
        id: &str,
    ) -> impl Future<Output = TursoResult<Option<GraphEntity>>> + Send;

    /// List entities, optionally filtered by type (parity of `list_entities`).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn list_entities(
        &self,
        entity_type: Option<&str>,
    ) -> impl Future<Output = TursoResult<Vec<GraphEntity>>> + Send;

    /// Find one entity by (type, exact name).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn find_entity(
        &self,
        entity_type: &str,
        name: &str,
    ) -> impl Future<Output = TursoResult<Option<GraphEntity>>> + Send;

    /// Add a relationship between two entities by allowlisted `rel_type`.
    /// Parity of `relate_dynamic`: unknown rels error; weight defaults to 1.0.
    ///
    /// # Errors
    /// Returns [`TursoError::Mapping`] on unknown rel; SQL errors otherwise.
    fn relate_dynamic(
        &self,
        from: &str,
        to: &str,
        rel_type: &str,
        weight: f64,
    ) -> impl Future<Output = TursoResult<()>> + Send;

    /// Delete one edge; returns rows removed.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn delete_edge(
        &self,
        from: &str,
        rel_type: &str,
        to: &str,
    ) -> impl Future<Output = TursoResult<u64>> + Send;

    /// Outgoing neighbors across ALL relation types.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn related_forward(
        &self,
        from_id: &str,
    ) -> impl Future<Output = TursoResult<Vec<GraphEdge>>> + Send;

    /// Incoming neighbors (reverse hop).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn related_backward(
        &self,
        to_id: &str,
    ) -> impl Future<Output = TursoResult<Vec<GraphEdge>>> + Send;

    /// Depth-capped transitive closure via recursive CTE (outgoing).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn traverse_closure(
        &self,
        start: &str,
        max_depth: u32,
    ) -> impl std::future::Future<Output = TursoResult<Vec<String>>> + Send;

    /// Outgoing relations with resolved target entities — parity of
    /// `graph::dynamic::get_related` (drives the named graph-query renderer).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn get_related(
        &self,
        from_id: &str,
        limit: usize,
    ) -> impl std::future::Future<Output = TursoResult<Vec<RelatedRow>>> + Send;

    /// Edges whose BOTH endpoints fall inside `node_ids` — parity of
    /// `graph::dynamic::list_edges_among` (drives the KG renderer).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn list_edges_among(
        &self,
        node_ids: &[String],
    ) -> impl std::future::Future<Output = TursoResult<Vec<EdgeRow>>> + Send;

    /// Every edge in the graph (migration validation + doctor).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn all_edges_raw(
        &self,
    ) -> impl std::future::Future<Output = TursoResult<Vec<(String, String, String)>>> + Send;
}

impl GraphStore for TursoStore {
    async fn upsert_entity(&self, e: &GraphEntity) -> TursoResult<()> {
        self.conn
            .execute(
                "INSERT INTO entities (id, entity_type, name, properties, content_hash, project) \
                 VALUES (?1,?2,?3,?4,?5,?6) \
                 ON CONFLICT(id) DO UPDATE SET \
                   entity_type=excluded.entity_type, name=excluded.name, \
                   properties=excluded.properties, content_hash=excluded.content_hash, \
                   project=excluded.project",
                libsql::params![
                    e.id.clone(),
                    e.entity_type.clone(),
                    e.name.clone(),
                    e.properties.clone(),
                    e.content_hash.clone(),
                    e.project.clone(),
                ],
            )
            .await
            .map_err(TursoError::from)?;
        Ok(())
    }

    async fn get_entity(&self, id: &str) -> TursoResult<Option<GraphEntity>> {
        let mut rows = self
            .query(
                "SELECT id, entity_type, name, properties, content_hash, project \
                 FROM entities WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await.map_err(TursoError::from)? {
            Some(row) => Ok(Some(entity_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn list_entities(&self, entity_type: Option<&str>) -> TursoResult<Vec<GraphEntity>> {
        let sql = match entity_type {
            Some(_) => {
                "SELECT id, entity_type, name, properties, content_hash, project \
                 FROM entities WHERE entity_type = ?1 ORDER BY id"
            }
            None => "SELECT id, entity_type, name, properties, content_hash, project \
                     FROM entities ORDER BY id",
        };
        let params = entity_type.map_or_else(Vec::new, |et| {
            vec![libsql::Value::Text(et.to_owned())]
        });
        let mut rows = self.query(sql, params).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            out.push(entity_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_entity(
        &self,
        entity_type: &str,
        name: &str,
    ) -> TursoResult<Option<GraphEntity>> {
        let mut rows = self
            .query(
                "SELECT id, entity_type, name, properties, content_hash, project \
                 FROM entities WHERE entity_type = ?1 AND name = ?2 LIMIT 1",
                libsql::params![entity_type, name],
            )
            .await?;
        match rows.next().await.map_err(TursoError::from)? {
            Some(row) => Ok(Some(entity_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn relate_dynamic(
        &self,
        from: &str,
        to: &str,
        rel_type: &str,
        weight: f64,
    ) -> TursoResult<()> {
        validate_rel(rel_type)?;
        self.conn
            .execute(
                "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
                 VALUES (?1,?2,?3,?4,?5) \
                 ON CONFLICT(from_id, rel_type, to_id) DO UPDATE SET weight=excluded.weight",
                libsql::params![from, rel_type, to, weight, super::now_rfc3339()],
            )
            .await
            .map_err(TursoError::from)?;
        Ok(())
    }

    async fn delete_edge(&self, from: &str, rel_type: &str, to: &str) -> TursoResult<u64> {
        self.conn
            .execute(
                "DELETE FROM entity_edges WHERE from_id=?1 AND rel_type=?2 AND to_id=?3",
                libsql::params![from, rel_type, to],
            )
            .await
            .map_err(TursoError::from)
    }

    async fn related_forward(&self, from_id: &str) -> TursoResult<Vec<GraphEdge>> {
        edges_query(
            self,
            "SELECT from_id, rel_type, to_id, weight FROM entity_edges WHERE from_id = ?1",
            from_id,
        )
        .await
    }

    async fn related_backward(&self, to_id: &str) -> TursoResult<Vec<GraphEdge>> {
        edges_query(
            self,
            "SELECT from_id, rel_type, to_id, weight FROM entity_edges WHERE to_id = ?1",
            to_id,
        )
        .await
    }

    async fn traverse_closure(&self, start: &str, max_depth: u32) -> TursoResult<Vec<String>> {
        let sql = format!(
            "WITH RECURSIVE reach(node, depth) AS ( \
               SELECT ?1, 0 \
               UNION \
               SELECT e.to_id, reach.depth + 1 FROM entity_edges e \
                 JOIN reach ON e.from_id = reach.node \
                 WHERE reach.depth < {max_depth} \
             ) SELECT DISTINCT node FROM reach WHERE node != ?1 ORDER BY node"
        );
        let mut rows = self.query(&sql, libsql::params![start]).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            out.push(row.get::<String>(0).map_err(TursoError::from)?);
        }
        Ok(out)
    }

    /// Outgoing relations with resolved target entities — parity of
    /// `graph::dynamic::get_related` (drives the named graph-query renderer).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    async fn get_related(
        &self,
        from_id: &str,
        limit: usize,
    ) -> TursoResult<Vec<RelatedRow>> {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "limit is a bounded search cap; far below i64::MAX"
        )]
        let limit64 = limit as i64;
        // Single join over ALL rel types (SurrealDB needed one query per
        // ALLOWED_RELS entry; libSQL stores rel_type as a column).
        let sql = "SELECT e.rel_type, e.weight, \
                   t.id, t.entity_type, t.name, t.properties, t.content_hash, t.project \
                   FROM entity_edges e JOIN entities t ON t.id = e.to_id \
                   WHERE e.from_id = ?1 LIMIT ?2";
        let mut rows = self
            .query(sql, libsql::params![from_id, limit64])
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            out.push(RelatedRow {
                rel_type: row.get::<String>(0).map_err(TursoError::from)?,
                weight: row.get::<f64>(1).map_err(TursoError::from)?,
                target: entity_from_row_offset(&row, 2)?,
            });
        }
        Ok(out)
    }

    /// Edges whose BOTH endpoints fall inside `node_ids` — parity of
    /// `graph::dynamic::list_edges_among` (drives the KG renderer).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    async fn list_edges_among(
        &self,
        node_ids: &[String],
    ) -> TursoResult<Vec<EdgeRow>> {
        if node_ids.len() < 2 {
            return Ok(Vec::new());
        }
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "placeholder index bounded by node_ids.len() (SQL param limit far higher)"
        )]
        let placeholders = node_ids.iter().enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT from_id, rel_type, to_id FROM entity_edges \
             WHERE from_id IN ({placeholders}) AND to_id IN ({placeholders}) \
             ORDER BY from_id, rel_type"
        );
        let mut params: Vec<libsql::Value> =
            node_ids.iter().map(|id| libsql::Value::Text(id.clone())).collect();
        params.extend(node_ids.iter().map(|id| libsql::Value::Text(id.clone())));
        let mut rows = self.query(&sql, params).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            out.push(EdgeRow {
                from: row.get::<String>(0).map_err(TursoError::from)?,
                rel_type: row.get::<String>(1).map_err(TursoError::from)?,
                to: row.get::<String>(2).map_err(TursoError::from)?,
            });
        }
        Ok(out)
    }

    async fn all_edges_raw(&self) -> TursoResult<Vec<(String, String, String)>> {
        let mut rows = self
            .query(
                "SELECT from_id, rel_type, to_id FROM entity_edges ORDER BY from_id",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            out.push((
                row.get::<String>(0).map_err(TursoError::from)?,
                row.get::<String>(1).map_err(TursoError::from)?,
                row.get::<String>(2).map_err(TursoError::from)?,
            ));
        }
        Ok(out)
    }
}

async fn edges_query(store: &TursoStore, sql: &str, id: &str) -> TursoResult<Vec<GraphEdge>> {
    let mut rows = store.query(sql, libsql::params![id]).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(TursoError::from)? {
        out.push(GraphEdge {
            from: row.get::<String>(0).map_err(TursoError::from)?,
            rel_type: row.get::<String>(1).map_err(TursoError::from)?,
            to: row.get::<String>(2).map_err(TursoError::from)?,
            weight: row.get::<f64>(3).map_err(TursoError::from)?,
        });
    }
    Ok(out)
}

fn entity_from_row(row: &libsql::Row) -> TursoResult<GraphEntity> {
    entity_from_row_offset(row, 0)
}

/// Entity mapping starting at column `base` (for joined rows where entity
/// columns don't start at 0).
fn entity_from_row_offset(row: &libsql::Row, base: i32) -> TursoResult<GraphEntity> {
    // Column indices are `base` plus compile-time constants 0..=5; base is a
    // literal (0 or 2) at every call site, so the adds cannot overflow.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "constant column offsets; base is 0 or 2 at all call sites"
    )]
    let cols = (0..6).map(|k| row.get::<Option<String>>(base + k).map_err(TursoError::from))
        .collect::<Result<Vec<_>, _>>()?;
    let mut it = cols.into_iter();
    Ok(GraphEntity {
        id: it.next().flatten().unwrap_or_default(),
        entity_type: it.next().flatten().unwrap_or_default(),
        name: it.next().flatten().unwrap_or_default(),
        properties: it.next().flatten(),
        content_hash: it.next().flatten(),
        project: it.next().flatten(),
    })
}

/// Render an entity in the Phase-0 golden fixture shape:
/// `[type] name (id=RecordId { table: Table("entity"), key: String("KEY") }) {props}`
#[must_use]
pub fn render_entity_fixture(e: &GraphEntity) -> String {
    let mut s = format!(
        "[{}] {} (id=RecordId {{ table: Table(\"entity\"), key: String(\"{}\") }})",
        e.entity_type, e.name, e.id
    );
    if let Some(props) = &e.properties {
        write!(s, " {props}").ok(); // String writes are infallible
    }
    s
}

/// Render one line of the `graph-query --list` output — parity of the RPC
/// `format_entity_line` (`[type] name (id=<debug-id>) [props]`).
#[must_use]
pub fn render_entity_line(e: &GraphEntity) -> String {
    let id_str = format!("RecordId {{ table: Table(\"entity\"), key: String(\"{}\") }}", e.id);
    match &e.properties {
        Some(p) if !p.is_empty() => {
            format!("[{}] {} (id={id_str}) {p}", e.entity_type, e.name)
        }
        _ => format!("[{}] {} (id={id_str})", e.entity_type, e.name),
    }
}

/// Render named-entity query lines — parity of `graph_query_named`:
/// header `[type] name (id=…)` then `  ──rel──► [type] name` per relation.
#[must_use]
pub fn render_named_lines(
    entity: &GraphEntity,
    related: &[RelatedRow],
) -> Vec<String> {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "capacity estimate: header line + related rows + sentinel; operands bounded"
    )]
    let mut lines = Vec::with_capacity(related.len() + 2);
    lines.push(format!(
        "[{}] {} (id=RecordId {{ table: Table(\"entity\"), key: String(\"{}\") }})",
        entity.entity_type, entity.name, entity.id
    ));
    if related.is_empty() {
        lines.push("  (no edges)".to_owned());
    } else {
        for r in related {
            let mut line = String::with_capacity(64);
            write!(
                line,
                "  ──{}──► [{}] {}",
                r.rel_type, r.target.entity_type, r.target.name
            )
            .ok();
            lines.push(line);
        }
    }
    lines
}

/// Allowlist accessor for tests/CLI listing.
#[must_use]
pub const fn allowed_rels() -> &'static [&'static str] {
    ALLOWED_RELS
}
