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

pub(crate) async fn open_direct_resilient()
-> Result<surrealdb::Surreal<surrealdb::engine::any::Any>, String> {
    let mut last = match kavach_surreal::open_default().await {
        Ok(db) => return Ok(db),
        Err(e) => e.to_string(),
    };
    for backoff in fallback_backoff_schedule() {
        if !is_rocksdb_lock_contention(&last) {
            break;
        }
        tokio::time::sleep(backoff).await;
        match kavach_surreal::open_default().await {
            Ok(db) => return Ok(db),
            Err(e) => last = e.to_string(),
        }
    }
    Err(last)
}
