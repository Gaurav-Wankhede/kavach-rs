//! Memory-entry storage — `EntryStore` extension trait over `TursoStore`.
//!
//! Parity surface of `read.rs` / `write.rs` / `delete.rs`: upsert (with FTS
//! sync via [`super::search`]), keyed get, list variants, status CAS,
//! priority/lane, events. Row mapping helpers are `pub(crate)` for reuse.

use std::future::Future;

use chrono::{DateTime, Utc};

use super::fake_record_id;
use super::{TursoError, TursoResult, TursoStore};
use crate::dual_write::MemoryEntry;

/// DDL owned by this domain, executed by `connection::init_schema`.
pub const DDL: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project, category)",
];

pub(crate) fn map_err<E: std::fmt::Display>(e: E) -> TursoError {
    TursoError::Mapping(format!("{e}"))
}

/// Column order MUST stay in sync with [`entry_from_row`] indices.
pub(crate) const ENTRY_COLS: &str = "project, category, entry_key, title, content, status, \
         tags, decay_score, access_count, entry_status, priority, lane, \
         exec_prompt, occupied_by, occupied_until, created_at, updated_at";

pub(crate) fn entry_from_row(row: &libsql::Row) -> TursoResult<MemoryEntry> {
    let get_s =
        |i: i32| -> Option<String> { row.get::<Option<String>>(i).ok().flatten() };
    let get_dt = |i: i32| -> Option<DateTime<Utc>> {
        get_s(i)
            .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
            .map(|d| d.with_timezone(&Utc))
    };
    let project: String =
        get_s(0).ok_or_else(|| TursoError::Mapping("memory row missing `project`".into()))?;
    let tags_json: String = get_s(6).unwrap_or_else(|| "[]".into());
    Ok(MemoryEntry {
        // RecordId is Surreal-only; the Turso DTO identity is the composite
        // (project, category, entry_key).
        id: None,
        project: fake_record_id(&project),
        category: get_s(1),
        entry_key: get_s(2).unwrap_or_default(),
        title: get_s(3).unwrap_or_default(),
        content: get_s(4).unwrap_or_default(),
        status: get_s(5),
        entry_status: get_s(9),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        decay_score: row.get::<Option<f64>>(7).ok().flatten(),
        access_count: row.get::<Option<i64>>(8).ok().flatten(),
        priority: row.get::<Option<i64>>(10).ok().flatten(),
        lane: get_s(11),
        exec_prompt: get_s(12),
        occupied_by: get_s(13),
        occupied_until: get_dt(14),
        created_at: get_dt(15),
        updated_at: get_dt(16),
    })
}

/// Memory-entry operations on the embedded store.
pub trait EntryStore {
    /// Insert-or-update a full entry (created_at preserved on update,
    /// updated_at bumped; FTS kept in sync).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn upsert_entry(
        &self,
        e: &MemoryEntry,
    ) -> impl Future<Output = TursoResult<()>> + Send;

    /// Fetch one entry by composite key.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn get_by_key(
        &self,
        project: &str,
        category: &str,
        key: &str,
    ) -> impl Future<Output = TursoResult<Option<MemoryEntry>>> + Send;

    /// All entries for a project, dispatch-ordered.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn list_by_project(
        &self,
        project: &str,
    ) -> impl Future<Output = TursoResult<Vec<MemoryEntry>>> + Send;

    /// Every entry across projects.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn list_all(&self) -> impl Future<Output = TursoResult<Vec<MemoryEntry>>> + Send;

    /// Entries in one category.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn list_by_category(
        &self,
        project: &str,
        category: &str,
    ) -> impl Future<Output = TursoResult<Vec<MemoryEntry>>> + Send;

    /// Filtered list mirroring `read.rs::list_with_filter`.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn list_with_filter(
        &self,
        project: &str,
        category: Option<&str>,
        status: Option<&str>,
        contains: Option<&str>,
        limit: i64,
    ) -> impl Future<Output = TursoResult<Vec<MemoryEntry>>> + Send;

    /// Update `status` + `entry_status` on one entry; returns rows affected.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn update_status(
        &self,
        project: &str,
        category: &str,
        key: &str,
        status: &str,
    ) -> impl Future<Output = TursoResult<u64>> + Send;

    /// Compare-and-set status transition (port of `update_status_cas`).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn update_status_cas(
        &self,
        project: &str,
        category: &str,
        key: &str,
        expect: &str,
        new: &str,
    ) -> impl Future<Output = TursoResult<bool>> + Send;

    /// Set (or clear) dispatch priority.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn set_priority(
        &self,
        project: &str,
        category: &str,
        key: &str,
        priority: Option<i64>,
    ) -> impl Future<Output = TursoResult<u64>> + Send;

    /// Set (or clear) the roadmap dispatch lane.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn set_lane(
        &self,
        project: &str,
        key: &str,
        lane: Option<String>,
    ) -> impl Future<Output = TursoResult<u64>> + Send;

    /// Append an event row.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn append_event(
        &self,
        project: &str,
        kind: &str,
        payload: &str,
    ) -> impl Future<Output = TursoResult<()>> + Send;

    /// Delete events older than `days`; returns rows removed.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn rotate_events(&self, days: i64) -> impl Future<Output = TursoResult<u64>> + Send;

    /// Delete one entry; also removes its FTS row.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn delete_by_key(
        &self,
        project: &str,
        category: &str,
        key: &str,
    ) -> impl Future<Output = TursoResult<u64>> + Send;

    /// Register or update a project in the projects table.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn register_project(
        &self,
        slug: &str,
        path: &str,
        tags: Option<&str>,
    ) -> impl Future<Output = TursoResult<()>> + Send;

    /// Add or update a citation for a project/entry_key.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn upsert_citation(
        &self,
        project: &str,
        entry_key: &str,
        name: &str,
        slug: &str,
        url: &str,
    ) -> impl Future<Output = TursoResult<()>> + Send;

    /// List all citations for a project, newest-updated first.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn list_citations(&self, project: &str) -> impl Future<Output = TursoResult<Vec<CitationRow>>> + Send;

    /// Get a gate config override.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn gate_config_get(
        &self,
        project: &str,
        gate_key: &str,
    ) -> impl Future<Output = TursoResult<Option<GateConfigRow>>> + Send;

    /// Set a gate config override.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn gate_config_set(
        &self,
        project: &str,
        gate_key: &str,
        kind: &str,
        num: Option<f64>,
        boolean: Option<bool>,
        list: Option<&str>,
        text: Option<&str>,
    ) -> impl Future<Output = TursoResult<()>> + Send;
}

/// A citation row for CLI display.
#[derive(Debug, Clone)]
pub struct CitationRow {
    pub entry_key: String,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub updated_at: String,
}

/// A gate config row for CLI display.
#[derive(Debug, Clone)]
pub struct GateConfigRow {
    pub project: String,
    pub gate_key: String,
    pub kind: String,
    pub value_num: Option<f64>,
    pub value_bool: Option<bool>,
    pub value_list: Option<String>,
    pub value_text: Option<String>,
    pub updated_at: String,
}

impl EntryStore for TursoStore {
    async fn upsert_entry(&self, e: &MemoryEntry) -> TursoResult<()> {
        let now = super::now_rfc3339();
        let project = record_id_slug(&e.project);
        let project_ref = project.clone();
        let tags = serde_json::to_string(&e.tags.clone().unwrap_or_default())
            .unwrap_or_else(|_| "[]".into());
        let category = e.category.clone().unwrap_or_default();
        self.conn
            .execute(
                "INSERT INTO memories (
                     project, category, entry_key, title, content, status,
                     tags, decay_score, access_count, entry_status, priority,
                     lane, exec_prompt, occupied_by, occupied_until,
                     created_at, updated_at)
                   VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
                   ON CONFLICT(project, category, entry_key) DO UPDATE SET
                     title=excluded.title,
                     content=excluded.content,
                     status=excluded.status,
                     tags=excluded.tags,
                     decay_score=excluded.decay_score,
                     access_count=excluded.access_count,
                     entry_status=excluded.entry_status,
                     priority=excluded.priority,
                     lane=excluded.lane,
                     exec_prompt=excluded.exec_prompt,
                     occupied_by=excluded.occupied_by,
                     occupied_until=excluded.occupied_until,
                     updated_at=excluded.updated_at",
                libsql::params![
                    project,
                    category.clone(),
                    e.entry_key.clone(),
                    e.title.clone(),
                    e.content.clone(),
                    e.status.clone(),
                    tags,
                    e.decay_score,
                    e.access_count.unwrap_or(0),
                    e.entry_status.clone(),
                    e.priority,
                    e.lane.clone(),
                    e.exec_prompt.clone(),
                    e.occupied_by.clone(),
                    e.occupied_until.map(|d| d.to_rfc3339()),
                    e.created_at.map_or_else(|| now.clone(), |d| d.to_rfc3339()),
                    now,
                ],
            )
            .await
            .map_err(map_err)?;
        // Keep FTS in sync (Phase 2).
        self.conn
            .execute(
                "DELETE FROM memories_fts WHERE project=?1 AND category=?2 AND entry_key=?3",
                libsql::params![project_ref.clone(), category.clone(), e.entry_key.clone()],
            )
            .await
            .map_err(map_err)?;
        self.conn
            .execute(
                "INSERT INTO memories_fts (entry_key, project, category, title, content) \
                 VALUES (?1,?2,?3,?4,?5)",
                libsql::params![
                    e.entry_key.clone(),
                    project_ref.clone(),
                    category.clone(),
                    e.title.clone(),
                    e.content.clone(),
                ],
            )
            .await
            .map_err(map_err)?;
        // Phase 5: write-time embedding — best-effort, never fails the upsert.
        // Block ONNX on a dedicated thread so the async executor stays unblocked.
        let title_for_embed = e.title.clone();
        let content_for_embed = e.content.clone();
        let embed_res = tokio::task::spawn_blocking(move || {
            crate::turso::vector::embed_one(&title_for_embed, &content_for_embed)
        })
        .await;
        if let Ok(Ok(vec)) = embed_res
            && vec.len() == crate::turso::vector::EMBEDDING_DIMS
        {
            let blob = crate::turso::vector::serialize_embedding(&vec);
            self.conn
                .execute(
                    "UPDATE memories SET embedding = ?1 WHERE project = ?2 AND category = ?3 AND entry_key = ?4",
                    libsql::params![blob, project_ref, category, e.entry_key.clone()],
                )
                .await
                .ok();
        }
        Ok(())
    }

    async fn get_by_key(
        &self,
        project: &str,
        category: &str,
        key: &str,
    ) -> TursoResult<Option<MemoryEntry>> {
        let sql = format!(
            "SELECT {ENTRY_COLS} FROM memories WHERE project=?1 AND category=?2 AND entry_key=?3"
        );
        let mut rows = self.query(&sql, libsql::params![project, category, key]).await?;
        take_first(&mut rows).await
    }

    async fn list_by_project(&self, project: &str) -> TursoResult<Vec<MemoryEntry>> {
        let sql = format!(
            "SELECT {ENTRY_COLS} FROM memories WHERE project=?1 \
             ORDER BY priority ASC NULLS LAST, created_at ASC"
        );
        collect(&mut self.query(&sql, libsql::params![project]).await?).await
    }

    async fn list_all(&self) -> TursoResult<Vec<MemoryEntry>> {
        let sql = format!("SELECT {ENTRY_COLS} FROM memories");
        collect(&mut self.query(&sql, ()).await?).await
    }

    async fn list_by_category(
        &self,
        project: &str,
        category: &str,
    ) -> TursoResult<Vec<MemoryEntry>> {
        let sql = format!(
            "SELECT {ENTRY_COLS} FROM memories WHERE project=?1 AND category=?2 \
             ORDER BY priority ASC NULLS LAST, created_at ASC"
        );
        collect(
            &mut self
                .query(&sql, libsql::params![project, category])
                .await?,
        )
        .await
    }

    async fn list_with_filter(
        &self,
        project: &str,
        category: Option<&str>,
        status: Option<&str>,
        contains: Option<&str>,
        limit: i64,
    ) -> TursoResult<Vec<MemoryEntry>> {
        // Build the positional arg list dynamically.
        use libsql::Value;
        fn text_args(project: &str, opts: [Option<&str>; 3]) -> Vec<Value> {
            let mut v = vec![Value::Text(project.to_owned())];
            for o in opts.into_iter().flatten() {
                v.push(Value::Text(o.to_owned()));
            }
            v
        }
        let args = text_args(project, [category, status, contains]);
        let mut where_ = "WHERE project=?1".to_owned();
        if category.is_some() {
            where_.push_str(" AND category=?2");
        }
        if status.is_some() {
            where_.push_str(" AND COALESCE(entry_status,status)=?3");
        }
        if contains.is_some() {
            where_.push_str(" AND (title LIKE '%'||?4||'%' OR content LIKE '%'||?4||'%')");
        }
        let sql = format!(
            "SELECT {ENTRY_COLS} FROM memories {where_} ORDER BY updated_at DESC LIMIT {limit}"
        );
        collect(&mut self.query(&sql, args).await?).await
    }

    async fn update_status(
        &self,
        project: &str,
        category: &str,
        key: &str,
        status: &str,
    ) -> TursoResult<u64> {
        self.conn
            .execute(
                "UPDATE memories SET status=?4, entry_status=?4, updated_at=?5 \
                 WHERE project=?1 AND category=?2 AND entry_key=?3",
                libsql::params![project, category, key, status, super::now_rfc3339()],
            )
            .await
            .map_err(map_err)
    }

    async fn update_status_cas(
        &self,
        project: &str,
        category: &str,
        key: &str,
        expect: &str,
        new: &str,
    ) -> TursoResult<bool> {
        let n = self
            .conn
            .execute(
                "UPDATE memories SET status=?5, entry_status=?5, updated_at=?6 \
                 WHERE project=?1 AND category=?2 AND entry_key=?3 AND status=?4",
                libsql::params![project, category, key, expect, new, super::now_rfc3339()],
            )
            .await
            .map_err(map_err)?;
        Ok(n > 0)
    }

    async fn set_priority(
        &self,
        project: &str,
        category: &str,
        key: &str,
        priority: Option<i64>,
    ) -> TursoResult<u64> {
        self.conn
            .execute(
                "UPDATE memories SET priority=?4 WHERE project=?1 AND category=?2 AND entry_key=?3",
                libsql::params![project, category, key, priority],
            )
            .await
            .map_err(map_err)
    }

    async fn set_lane(
        &self,
        project: &str,
        key: &str,
        lane: Option<String>,
    ) -> TursoResult<u64> {
        self.conn
            .execute(
                "UPDATE memories SET lane=?3 WHERE project=?1 AND category='roadmap' AND entry_key=?2",
                libsql::params![project, key, lane],
            )
            .await
            .map_err(map_err)
    }

    async fn append_event(&self, project: &str, kind: &str, payload: &str) -> TursoResult<()> {
        self.conn
            .execute(
                "INSERT INTO events (project, kind, payload, created_at) VALUES (?1,?2,?3,?4)",
                libsql::params![project, kind, payload, super::now_rfc3339()],
            )
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn rotate_events(&self, days: i64) -> TursoResult<u64> {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "days comes from the CLI; negative values simply archive nothing"
        )]
        let cutoff_secs = Utc::now().timestamp() - days * 86_400;
        let cutoff = DateTime::<Utc>::from_timestamp(cutoff_secs, 0)
            .unwrap_or_else(Utc::now)
            .to_rfc3339();
        self.conn
            .execute("DELETE FROM events WHERE created_at < ?1", libsql::params![cutoff])
            .await
            .map_err(map_err)
    }

    async fn delete_by_key(
        &self,
        project: &str,
        category: &str,
        key: &str,
    ) -> TursoResult<u64> {
        let n = self
            .conn
            .execute(
                "DELETE FROM memories WHERE project=?1 AND category=?2 AND entry_key=?3",
                libsql::params![project, category, key],
            )
            .await
            .map_err(map_err)?;
        self.conn
            .execute(
                "DELETE FROM memories_fts WHERE project=?1 AND category=?2 AND entry_key=?3",
                libsql::params![project, category, key],
            )
            .await
            .map_err(map_err)?;
        Ok(n)
    }

    async fn register_project(
        &self,
        slug: &str,
        path: &str,
        tags: Option<&str>,
    ) -> TursoResult<()> {
        let tags_json = tags.unwrap_or("[]");
        self.conn
            .execute(
                "INSERT INTO projects (slug, path, tags) VALUES (?1, ?2, ?3)
                 ON CONFLICT(slug) DO UPDATE SET path=excluded.path, tags=excluded.tags",
                libsql::params![slug, path, tags_json],
            )
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn upsert_citation(
        &self,
        project: &str,
        entry_key: &str,
        name: &str,
        slug: &str,
        url: &str,
    ) -> TursoResult<()> {
        let now = super::now_rfc3339();
        let now2 = now.clone();
        self.conn
            .execute(
                "INSERT INTO citations (project, entry_key, name, slug, url, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(project, entry_key) DO UPDATE SET
                   name=excluded.name, slug=excluded.slug, url=excluded.url, updated_at=excluded.updated_at",
                libsql::params![project, entry_key, name, slug, url, now, now2],
            )
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn list_citations(&self, project: &str) -> TursoResult<Vec<CitationRow>> {
        let mut rows = self
            .conn
            .query(
                "SELECT entry_key, name, slug, url, updated_at FROM citations WHERE project = ?1 ORDER BY updated_at DESC",
                libsql::params![project],
            )
            .await
            .map_err(map_err)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            out.push(CitationRow {
                entry_key: row.get::<String>(0).map_err(map_err)?,
                name: row.get::<String>(1).map_err(map_err)?,
                slug: row.get::<String>(2).map_err(map_err)?,
                url: row.get::<String>(3).map_err(map_err)?,
                updated_at: row.get::<String>(4).map_err(map_err)?,
            });
        }
        Ok(out)
    }

    async fn gate_config_get(
        &self,
        project: &str,
        gate_key: &str,
    ) -> TursoResult<Option<GateConfigRow>> {
        let mut rows = self
            .conn
            .query(
                "SELECT project, gate_key, kind, value_num, value_bool, value_list, value_text, updated_at
                 FROM gate_config WHERE project = ?1 AND gate_key = ?2",
                libsql::params![project, gate_key],
            )
            .await
            .map_err(map_err)?;
        if let Some(row) = rows.next().await.map_err(TursoError::from)? {
            let value_num: Option<f64> = row.get(3).map_err(map_err).ok().flatten();
            let value_bool: Option<bool> = row.get::<Option<i64>>(4).map_err(map_err).ok().flatten().map(|v| v != 0);
            Ok(Some(GateConfigRow {
                project: row.get::<String>(0).map_err(map_err)?,
                gate_key: row.get::<String>(1).map_err(map_err)?,
                kind: row.get::<String>(2).map_err(map_err)?,
                value_num,
                value_bool,
                value_list: row.get::<Option<String>>(5).map_err(map_err).ok().flatten(),
                value_text: row.get::<Option<String>>(6).map_err(map_err).ok().flatten(),
                updated_at: row.get::<String>(7).map_err(map_err)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn gate_config_set(
        &self,
        project: &str,
        gate_key: &str,
        kind: &str,
        num: Option<f64>,
        boolean: Option<bool>,
        list: Option<&str>,
        text: Option<&str>,
    ) -> TursoResult<()> {
        let now = super::now_rfc3339();
        let value_bool = boolean.map(|b| if b { 1i64 } else { 0i64 });
        self.conn
            .execute(
                "INSERT INTO gate_config (project, gate_key, kind, value_num, value_bool, value_list, value_text, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(project, gate_key) DO UPDATE SET
                   kind=excluded.kind, value_num=excluded.value_num, value_bool=excluded.value_bool,
                   value_list=excluded.value_list, value_text=excluded.value_text, updated_at=excluded.updated_at",
                libsql::params![project, gate_key, kind, num, value_bool, list, text, now],
            )
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

/// Extract the slug from the Surreal RecordId we synthesize at read time.
fn record_id_slug(id: &surrealdb_types::RecordId) -> String {
    crate::key_str::project_key_str(id)
}

async fn collect(rows: &mut libsql::Rows) -> TursoResult<Vec<MemoryEntry>> {
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(TursoError::from)? {
        out.push(entry_from_row(&row)?);
    }
    Ok(out)
}

async fn take_first(rows: &mut libsql::Rows) -> TursoResult<Option<MemoryEntry>> {
    match rows.next().await.map_err(TursoError::from)? {
        Some(row) => Ok(Some(entry_from_row(&row)?)),
        None => Ok(None),
    }
}
