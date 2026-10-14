pub(crate) fn or_str(opt: Option<String>, default: &str) -> String {
    if let Some(s) = opt {
        return s;
    }
    default.to_owned()
}

pub(crate) fn is_rocksdb_lock_contention(open_err: &str) -> bool {
    open_err.contains("Resource temporarily unavailable") || open_err.contains("LOCK:")
}

pub(crate) fn fallback_backoff_schedule() -> impl Iterator<Item = std::time::Duration> {
    [100u64, 250, 500, 1000, 1500]
        .into_iter()
        .map(std::time::Duration::from_millis)
}

/// Open the Database (libSQL) store — Phase 6 default direct-open path.
///
/// libSQL is a local file; no lock-contention retry needed.
pub(crate) async fn open_turso_resilient() -> Result<kavach_db::db::DbStore, String> {
    kavach_db::db::DbStore::open_default()
        .await
        .map_err(|e| e.to_string())
}

/// Legacy alias for `open_database_resilient` — the Database RocksDB retry loop
/// is obsolete; libSQL opens are local-file and non-contended.
pub(crate) async fn open_direct_resilient() -> Result<kavach_db::db::DbStore, String> {
    open_turso_resilient().await
}
