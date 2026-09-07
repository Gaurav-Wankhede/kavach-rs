// split: intentional - read helpers for algo_decision and arch_decision tables
// sql-safe: queries use static literals + params; no user input concatenated.
use crate::error::Result;
use crate::turso::TursoStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AlgoDecision {
    pub id: Option<String>,
    pub project: String,
    pub problem_class: String,
    pub chosen: String,
    pub time_complexity: String,
    pub space_complexity: String,
    pub file_path: String,
    pub search_year: i32,
    pub search_month: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArchDecision {
    pub id: Option<String>,
    pub project: String,
    pub pattern: String,
    pub scope: String,
    pub cap_choice: Option<String>,
    pub failure_mode: String,
    pub tradeoff: String,
    pub file_path: String,
    pub search_year: i32,
    pub search_month: i32,
}

/// List up to `limit` recent `algo_decision` rows for `project_id`.
///
/// # Errors
/// Propagates `Error::Turso` from the SELECT.
pub async fn algo_list_recent(
    db: &TursoStore,
    project_id: &str,
    limit: usize,
) -> Result<Vec<AlgoDecision>> {
    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let mut rows = db
        .exec_params(
            "SELECT id, project, problem_class, chosen, time_complexity, space_complexity, \
                    file_path, search_year, search_month \
             FROM algo_decision WHERE project = $project LIMIT $limit",
            libsql::params![project_id.to_owned(), limit_i64],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(AlgoDecision {
            id: Some(r.get::<String>(0)?),
            project: r.get::<String>(1)?,
            problem_class: r.get::<String>(2)?,
            chosen: r.get::<String>(3)?,
            time_complexity: r.get::<String>(4)?,
            space_complexity: r.get::<String>(5)?,
            file_path: r.get::<String>(6)?,
            search_year: i32::try_from(r.get::<i64>(7)?).unwrap_or_default(),
            search_month: i32::try_from(r.get::<i64>(8)?).unwrap_or_default(),
        });
    }
    Ok(out)
}

#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct ArchUpsertParams<'a> {
    pub project: String,
    pub pattern: &'a str,
    pub scope: &'a str,
    pub cap_choice: Option<&'a str>,
    pub failure_mode: &'a str,
    pub tradeoff: &'a str,
    pub file_path: &'a str,
    pub search_year: i32,
    pub search_month: i32,
}

#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc)"
)]
pub struct AlgoUpsertParams<'a> {
    pub project: String,
    pub problem_class: &'a str,
    pub chosen: &'a str,
    pub time_complexity: &'a str,
    pub space_complexity: &'a str,
    pub file_path: &'a str,
    pub search_year: i32,
    pub search_month: i32,
}

/// Upsert an `arch_decision` row keyed by (project, pattern, `file_path`).
///
/// # Errors
/// `Error::Turso` on query failure; `Error::RecordNotFound` if the upsert
/// yields no id row.
pub async fn arch_upsert(db: &TursoStore, p: &ArchUpsertParams<'_>) -> Result<String> {
    // Single-statement keyed UPSERT, NOT racy DELETE;CREATE (sibling of
    // algo_upsert). SOURCE: decision.algo-upsert-idempotent-keyed.
    let key = arch_record_key(&p.project, p.pattern, p.file_path);
    let mut resp = db
        .exec_params(
            "INSERT INTO arch_decision \
                 (id, project, pattern, scope, cap_choice, failure_mode, tradeoff, \
                  file_path, search_year, search_month) \
             VALUES ($key, $project, $pattern, $scope, $cap, $failure_mode, $tradeoff, \
                     $file_path, $search_year, $search_month) \
             ON CONFLICT(id) DO UPDATE SET \
                 project = excluded.project, pattern = excluded.pattern, \
                 scope = excluded.scope, cap_choice = excluded.cap_choice, \
                 failure_mode = excluded.failure_mode, tradeoff = excluded.tradeoff, \
                 file_path = excluded.file_path, search_year = excluded.search_year, \
                 search_month = excluded.search_month \
             RETURNING id",
            libsql::params![
                key.clone(),
                p.project.clone(),
                p.pattern.to_owned(),
                p.scope.to_owned(),
                p.cap_choice.map(ToOwned::to_owned),
                p.failure_mode.to_owned(),
                p.tradeoff.to_owned(),
                p.file_path.to_owned(),
                p.search_year,
                p.search_month
            ],
        )
        .await?;
    match resp.next().await? {
        Some(_) => Ok(key),
        None => Err(crate::error::Error::RecordNotFound(
            "arch_decision upsert returned no id".into(),
        )),
    }
}

/// Upsert an `algo_decision` row keyed by (project, `problem_class`,
/// `file_path`).
///
/// # Errors
/// `Error::Turso` on query failure; `Error::RecordNotFound` if the upsert
/// yields no id row.
pub async fn algo_upsert(db: &TursoStore, p: &AlgoUpsertParams<'_>) -> Result<String> {
    // Single-statement keyed UPSERT, NOT racy DELETE;CREATE (two concurrent
    // recorders -> double row; reader mid-swap -> zero rows).
    // SOURCE: decision.algo-upsert-idempotent-keyed.
    let key = algo_record_key(&p.project, p.problem_class, p.file_path);
    let mut resp = db
        .exec_params(
            "INSERT INTO algo_decision \
                 (id, project, problem_class, chosen, time_complexity, space_complexity, \
                  file_path, search_year, search_month) \
             VALUES ($key, $project, $problem_class, $chosen, $time, $space, \
                     $file_path, $search_year, $search_month) \
             ON CONFLICT(id) DO UPDATE SET \
                 project = excluded.project, problem_class = excluded.problem_class, \
                 chosen = excluded.chosen, time_complexity = excluded.time_complexity, \
                 space_complexity = excluded.space_complexity, \
                 file_path = excluded.file_path, search_year = excluded.search_year, \
                 search_month = excluded.search_month \
             RETURNING id",
            libsql::params![
                key.clone(),
                p.project.clone(),
                p.problem_class.to_owned(),
                p.chosen.to_owned(),
                p.time_complexity.to_owned(),
                p.space_complexity.to_owned(),
                p.file_path.to_owned(),
                p.search_year,
                p.search_month
            ],
        )
        .await?;
    match resp.next().await? {
        Some(_) => Ok(key),
        None => Err(crate::error::Error::RecordNotFound(
            "algo_decision upsert returned no id".into(),
        )),
    }
}

/// Deterministic `algo_decision` record key: `blake3(project:class:file)[..32]`.
/// Identical inputs -> identical key, so the UPSERT is idempotent per dedup
/// tuple — the same recipe `graph/mistakes/cluster.rs` uses for its content key.
fn algo_record_key(project: &str, problem_class: &str, file_path: &str) -> String {
    hash_decision_key(project, problem_class, file_path)
}

/// Deterministic `arch_decision` record key — sibling of `algo_record_key`.
fn arch_record_key(project: &str, pattern: &str, file_path: &str) -> String {
    hash_decision_key(project, pattern, file_path)
}

/// Pure seed→key hash for a (project, discriminant, file) decision tuple, split
/// out so determinism is unit-testable without a DB. Shared by algo + arch
/// upserts — both key one row per `(project, <pattern|class>, file_path)`.
pub(crate) fn hash_decision_key(project_key: &str, discriminant: &str, file_path: &str) -> String {
    hash_keyed("decision", project_key, discriminant, file_path)
}

/// Table-namespaced deterministic record-id hash: `blake3(table:a:b:c)[..32]`.
/// The `table` prefix keeps each table's keyspace independent (no cross-table
/// seed aliasing); 32 hex chars = 128 bits puts the birthday bound far beyond
/// any realistic row count, so a key collision is not a practical concern.
pub(crate) fn hash_keyed(table: &str, a: &str, b: &str, c: &str) -> String {
    let seed = format!("{table}:{a}:{b}:{c}");
    let hex = blake3::hash(seed.as_bytes()).to_hex();
    // blake3 hex is ASCII, so [..32] is on a char boundary; .get is panic-free.
    hex.get(..32).unwrap_or(&hex).to_owned()
}

/// List up to `limit` recent `arch_decision` rows for `project_id`.
///
/// # Errors
/// Propagates `Error::Turso` from the SELECT.
pub async fn arch_list_recent(
    db: &TursoStore,
    project_id: &str,
    limit: usize,
) -> Result<Vec<ArchDecision>> {
    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let mut rows = db
        .exec_params(
            "SELECT id, project, pattern, scope, cap_choice, failure_mode, tradeoff, \
                    file_path, search_year, search_month \
             FROM arch_decision WHERE project = $project LIMIT $limit",
            libsql::params![project_id.to_owned(), limit_i64],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(ArchDecision {
            id: Some(r.get::<String>(0)?),
            project: r.get::<String>(1)?,
            pattern: r.get::<String>(2)?,
            scope: r.get::<String>(3)?,
            cap_choice: r.get::<Option<String>>(4)?,
            failure_mode: r.get::<String>(5)?,
            tradeoff: r.get::<String>(6)?,
            file_path: r.get::<String>(7)?,
            search_year: i32::try_from(r.get::<i64>(8)?).unwrap_or_default(),
            search_month: i32::try_from(r.get::<i64>(9)?).unwrap_or_default(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod algo_upsert_tests {
    use super::{AlgoUpsertParams, algo_list_recent, algo_upsert};
    use crate::open_memory;

    // The whole point of the keyed-UPSERT race fix: two upserts of the same
    // (project, problem_class, file_path) tuple converge to ONE row carrying
    // the latest value — never two rows (the old DELETE;CREATE double-count).
    #[tokio::test]
    async fn re_upsert_same_tuple_converges_to_one_row() {
        let db = open_memory().await.expect("open mem");
        let pid = "p".to_owned();
        let mk = |chosen: &'static str| AlgoUpsertParams {
            project: pid.clone(),
            problem_class: "sort",
            chosen,
            time_complexity: "O(n log n)",
            space_complexity: "O(1)",
            file_path: "src/x.rs",
            search_year: 2026,
            search_month: 6,
        };
        algo_upsert(&db, &mk("quicksort"))
            .await
            .expect("first upsert");
        algo_upsert(&db, &mk("heapsort"))
            .await
            .expect("second upsert");

        let rows = algo_list_recent(&db, &pid, 10).await.expect("list");
        assert_eq!(
            rows.len(),
            1,
            "re-upsert must converge to one row, not duplicate"
        );
        assert_eq!(rows[0].chosen, "heapsort", "row carries the latest value");
    }

    // A different tuple is a distinct row — the dedup key must not over-merge.
    #[tokio::test]
    async fn distinct_tuple_is_a_separate_row() {
        let db = open_memory().await.expect("open mem");
        let pid = "p".to_owned();
        let mk = |file_path: &'static str| AlgoUpsertParams {
            project: pid.clone(),
            problem_class: "sort",
            chosen: "quicksort",
            time_complexity: "O(n log n)",
            space_complexity: "O(1)",
            file_path,
            search_year: 2026,
            search_month: 6,
        };
        algo_upsert(&db, &mk("src/x.rs")).await.expect("base");
        algo_upsert(&db, &mk("src/y.rs")).await.expect("other");
        let rows = algo_list_recent(&db, &pid, 10).await.expect("list");
        assert_eq!(rows.len(), 2, "distinct file_path -> distinct row");
    }
}

#[cfg(test)]
mod algo_key_tests {
    use super::hash_decision_key;

    #[test]
    fn same_tuple_yields_same_key() {
        // Idempotency proof: identical (project, discriminant, file) -> identical
        // id, so the UPSERT collapses concurrent recordings to one row.
        let a = hash_decision_key("p1", "sort", "src/x.rs");
        let b = hash_decision_key("p1", "sort", "src/x.rs");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32); // 128-bit key (32 hex chars)
    }

    #[test]
    fn distinct_tuples_yield_distinct_keys() {
        let base = hash_decision_key("p1", "sort", "src/x.rs");
        assert_ne!(base, hash_decision_key("p2", "sort", "src/x.rs"));
        assert_ne!(base, hash_decision_key("p1", "search", "src/x.rs"));
        assert_ne!(base, hash_decision_key("p1", "sort", "src/y.rs"));
    }
}
