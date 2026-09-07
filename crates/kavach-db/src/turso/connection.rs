//! Connection lifecycle + schema bootstrap for [`TursoStore`].
//!
//! The ONE inherent impl block for `TursoStore` lives here (repo law); all
//! domain behavior is provided by extension traits in sibling modules
//! (`entries::EntryStore`, `search::SearchStore`, `graph::GraphStore`).

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use super::{SCHEMA_DDL, TursoError, TursoResult, TursoStore};

#[derive(Debug)]
pub struct QueryBuilder {
    store: TursoStore,
    sql: String,
    params: Vec<libsql::Value>,
}

impl QueryBuilder {
    #[must_use]
    pub fn bind(mut self, pair: (&str, impl Into<libsql::Value>)) -> Self {
        self.params.push(pair.1.into());
        self
    }
}

impl IntoFuture for QueryBuilder {
    type Output = TursoResult<libsql::Rows>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.store
                .conn
                .query(&self.sql, self.params)
                .await
                .map_err(Into::into)
        })
    }
}

impl TursoStore {
    pub fn query(&self, sql: impl AsRef<str>) -> QueryBuilder {
        QueryBuilder {
            store: self.clone(),
            sql: sql.as_ref().to_owned(),
            params: Vec::new(),
        }
    }

    /// Run a parameterized query, returning the raw `libsql::Rows`.
    ///
    /// # Errors
    /// Returns `TursoError` when the statement fails to prepare or execute.
    pub async fn query_with_params(
        &self,
        sql: &str,
        params: impl libsql::params::IntoParams,
    ) -> TursoResult<libsql::Rows> {
        self.conn.query(sql, params).await.map_err(Into::into)
    }

    /// Open (or create) a local Turso database at `path`, applying the schema.
    ///
    /// # Errors
    /// Returns `TursoError` when the file cannot be created, the connection
    /// cannot be established, or schema bootstrap fails.
    pub async fn open(path: impl AsRef<Path>) -> TursoResult<Self> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await.map_err(|e| {
                TursoError::Mapping(format!("create_dir_all {}: {e}", dir.display()))
            })?;
        }
        let db = libsql::Builder::new_local(path.display().to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        let store = Self { conn };
        store.init_schema().await?;
        Ok(store)
    }

    /// Default location: `$KAVACH_HOME/kavach.db`, else `~/.kavach/kavach.db`.
    ///
    /// # Errors
    /// Returns [`TursoError`] on filesystem or SQL failure.
    pub async fn open_default() -> TursoResult<Self> {
        let base = std::env::var_os("KAVACH_HOME").map_or_else(
            || {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".kavach")
            },
            std::path::PathBuf::from,
        );
        Self::open(base.join(super::DEFAULT_DB_FILENAME)).await
    }

    /// Open an in-memory store (tests). The single held connection keeps the
    /// `:memory:` database alive for the store's lifetime.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    pub async fn open_memory() -> TursoResult<Self> {
        let db = libsql::Builder::new_local(":memory:").build().await?;
        let conn = db.connect()?;
        let store = Self { conn };
        store.init_schema().await?;
        Ok(store)
    }

    /// Re-apply the schema idempotently (all DDL uses `IF NOT EXISTS`).
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    pub async fn apply_schema(&self) -> TursoResult<()> {
        self.init_schema().await
    }

    /// Run WAL checkpoint on the database.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    pub async fn wal_checkpoint(&self, truncate: bool) -> TursoResult<()> {
        let sql = if truncate {
            "PRAGMA wal_checkpoint(TRUNCATE);"
        } else {
            "PRAGMA wal_checkpoint(PASSIVE);"
        };
        self.exec_drain(sql).await
    }

    /// Run PRAGMA integrity_check on the database.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure or if integrity check fails.
    pub async fn integrity_check(&self) -> TursoResult<Vec<String>> {
        let mut rows = self.conn.query("PRAGMA integrity_check;", ()).await.map_err(TursoError::from)?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            if let Ok(msg) = row.get::<String>(0) {
                results.push(msg);
            }
        }
        Ok(results)
    }

    pub(crate) async fn exec(&self, sql: &str) -> TursoResult<u64> {
        self.conn.execute(sql, ()).await.map_err(Into::into)
    }

    pub(crate) async fn exec_params(
        &self,
        sql: impl AsRef<str>,
        params: impl libsql::params::IntoParams,
    ) -> TursoResult<libsql::Rows> {
        self.conn
            .query(sql.as_ref(), params)
            .await
            .map_err(Into::into)
    }

    /// Execute statements that may return rows (e.g. `PRAGMA journal_mode`)
    /// by draining the result set.
    async fn exec_drain(&self, sql: &str) -> TursoResult<()> {
        let mut rows = self.conn.query(sql, ()).await.map_err(TursoError::from)?;
        while rows.next().await.map_err(TursoError::from)?.is_some() {}
        Ok(())
    }

    async fn init_schema(&self) -> TursoResult<()> {
        self.exec_drain("PRAGMA journal_mode=WAL;").await?;
        self.exec_drain("PRAGMA wal_autocheckpoint=512;").await?; // §7 ops discipline
        self.exec_drain("PRAGMA busy_timeout=5000;").await?;
        for stmt in SCHEMA_DDL {
            self.exec(stmt).await?;
        }
        for stmt in entries_ddl().chain(search_ddl()).chain(graph_ddl()) {
            self.exec(stmt).await?;
        }
        for stmt in vector_ddl() {
            crate::turso::vector::exec_vector_ddl_best_effort(self, stmt).await?;
        }
        Ok(())
    }
}

fn entries_ddl() -> impl Iterator<Item = &'static str> {
    super::entries::DDL.iter().copied()
}

fn search_ddl() -> impl Iterator<Item = &'static str> {
    super::search::DDL.iter().copied()
}

fn graph_ddl() -> impl Iterator<Item = &'static str> {
    super::graph::DDL.iter().copied()
}

fn vector_ddl() -> impl Iterator<Item = &'static str> {
    super::vector::DDL.iter().copied()
}
