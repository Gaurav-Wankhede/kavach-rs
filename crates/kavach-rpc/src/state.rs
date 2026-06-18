use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use tokio::sync::Notify;

/// In-process change channel: a monotonic version counter plus a `Notify`.
///
/// The LIVE SELECT watcher task bumps `version` and wakes all `change.wait`
/// callers on every committed DB change. The GUI long-polls `change.wait`, so
/// it re-fetches ONLY when real data changed — event-driven, no idle polling.
/// SOURCE: research.poll-vs-event-gui · surrealdb LIVE SELECT.
#[derive(Debug, Default)]
pub struct ChangeFeed {
    version: AtomicU64,
    notify: Notify,
}

impl ChangeFeed {
    /// Current change version. Monotonically increases on every DB change.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Record a change: increment the version and wake every waiter.
    pub fn bump(&self) {
        self.version.fetch_add(1, Ordering::Release);
        self.notify.notify_waiters();
    }

    /// Wait until the version advances past `since`, or `timeout` elapses.
    /// Returns the current version either way (callers diff it against `since`).
    ///
    /// Subscribing to `notified()` BEFORE the version check closes the race
    /// where a bump lands between the check and the await.
    pub async fn wait_past(&self, since: u64, timeout: Duration) -> u64 {
        loop {
            let notified = self.notify.notified();
            let current = self.version();
            if current > since {
                return current;
            }
            // Timed out with no new change → return current (== since) so the
            // caller long-polls again. Never blocks a worker indefinitely.
            if tokio::time::timeout(timeout, notified).await.is_err() {
                return self.version();
            }
        }
    }
}

#[derive(Clone, Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at RPC handler boundary"
)]
pub struct AppState {
    pub db: Arc<Surreal<Db>>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Shared change feed driven by the LIVE SELECT watcher, awaited by
    /// the `change.wait` RPC. Cloned cheaply (Arc) across handlers + the task.
    pub changes: Arc<ChangeFeed>,
}

impl AppState {
    #[must_use]
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            db: Arc::new(db),
            started_at: chrono::Utc::now(),
            changes: Arc::new(ChangeFeed::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_past_returns_immediately_when_already_advanced() {
        let feed = ChangeFeed::default();
        feed.bump(); // version = 1
        let v = feed.wait_past(0, Duration::from_secs(5)).await;
        assert_eq!(v, 1, "already-newer version returns without blocking");
    }

    #[tokio::test]
    async fn wait_past_times_out_to_same_version() {
        let feed = ChangeFeed::default();
        let v = feed.wait_past(0, Duration::from_millis(20)).await;
        assert_eq!(v, 0, "timeout with no change returns the unchanged version");
    }

    #[tokio::test]
    async fn wait_past_wakes_on_bump() {
        let feed = Arc::new(ChangeFeed::default());
        let waiter = {
            let f = Arc::clone(&feed);
            tokio::spawn(async move { f.wait_past(0, Duration::from_secs(5)).await })
        };
        // Give the waiter a tick to subscribe, then bump.
        tokio::time::sleep(Duration::from_millis(10)).await;
        feed.bump();
        let v = waiter.await.expect("waiter task");
        assert_eq!(v, 1, "a bump wakes the waiter with the new version");
    }
}
