use std::fs;
use std::io;

use fs2::FileExt;

use crate::paths::{ensure_parent_dir, state_path_for};
use crate::state::SessionState;

impl SessionState {
    /// Save session state to INI file with file locking + atomic rename.
    ///
    /// # Errors
    ///
    /// Returns `Err` if parent directory creation, file locking, write, or rename fails.
    pub fn save(&self) -> io::Result<()> {
        let path = state_path_for(&self.session_id);
        ensure_parent_dir(&path)?;

        let lock_path = path.with_extension("lock");
        ensure_parent_dir(&lock_path)?;
        let lock_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let tmp_path = path.with_extension("tmp");
        let content = self.to_ini_full();
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path)?;

        // Write-through UNDER the lock, before unlock — see atomic_update for
        // the lost-update rationale: the DB write must serialize in the same
        // order as the file commit, or two racing saves diverge the stores.
        self.write_through_to_db(&content);
        lock_file.unlock()?;
        Ok(())
    }

    // ARCH: lock-ordered mirror to durable store
    // BOTTLENECK: state persistence must keep two stores (INI file, DB row)
    //   in the SAME commit order under concurrent writers (parallel hooks).
    // CAPACITY: save() fires ~1-5x/turn; INI write ~50us, RPC upsert ~1-3ms.
    //   The RPC runs UNDER the same file lock that orders the INI write — the
    //   caller pays ~1-3ms more lock-hold per save, bounded and acceptable.
    // CAP: CP within a session_id — both stores commit in lock order, so the
    //   DB row never lags the INI file. The INI is itself on-disk durable
    //   (atomic rename); this is NOT lossy in-memory write-back. INI =
    //   hot-path cache; DB = durable cross-session queryability + the
    //   session_id-keyed drift fix.
    //   {"name":"write-through AFTER unlock","reason":"lost update — two racing saves commit to the INI in one order, to the DB in the other; the DB row goes stale vs the file (caught in review)"},
    //   {"name":"DB-only, drop the INI","reason":"mandatory RPC round-trip on every hook; server outage = no state persistence at all"}
    // ]
    // TIME: O(1) per save | SPACE: O(state size) — one blob
    // YEAR: 2026 | SEARCHED: 2026-05
    //   the next successful save() reconciles it (idempotent upsert). Acceptable
    //   — load prefers the DB but the INI fallback is gated on session_id, so a
    //   stale DB row cannot cause cross-session drift. Fail-open: a dead server
    //   is logged, never propagated — it must not break a gate's save().
    // SOURCE: https://aws.amazon.com/caching/best-practices/
    //
    /// Best-effort durable mirror to the `session_runtime` `SurrealDB` table,
    /// keyed by `session_id`. The INI file write above already committed and is
    /// the local hot-path cache; this makes the DB the durable + queryable
    /// truth so state survives `/clear` (a new `session_id` reads its OWN row,
    /// never a prior conversation's). Fail-open: a missing `session_id` or a
    /// dead RPC server is logged, not propagated.
    fn write_through_to_db(&self, ini_content: &str) {
        if self.session_id.is_empty() {
            return;
        }
        let params = serde_json::json!({
            "session_id": self.session_id,
            "workdir": self.work_dir,
            "state_blob": ini_content,
        });
        if let Err(e) = kavach_rpc::client::call::<_, bool>("session.upsert", Some(params)) {
            tracing::warn!(error = ?e, "kavach-session: DB write-through failed (INI cache committed)");
        }
    }

    /// Save session state, logging any I/O error via tracing instead of propagating.
    pub fn save_or_log(&mut self) {
        if let Err(e) = self.save() {
            tracing::warn!(error = %e, "kavach-session: save failed");
        }
    }

    /// Atomic read-modify-write under exclusive lock.
    ///
    /// Reloads fresh state under lock, applies mutator, atomic-rename commits.
    /// SOURCE: CWE-367 TOCTOU mitigation — single exclusive lock window across
    /// the load+modify+save cycle prevents lost updates from parallel hooks.
    ///
    /// # Errors
    ///
    /// Returns `Err` if parent directory creation, file locking, read, write, or rename fails.
    pub fn atomic_update<F>(&mut self, mutator: F) -> io::Result<()>
    where
        F: FnOnce(&mut Self),
    {
        let path = state_path_for(&self.session_id);
        ensure_parent_dir(&path)?;

        let lock_path = path.with_extension("lock");
        ensure_parent_dir(&lock_path)?;
        let lock_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        if let Ok(content) = fs::read_to_string(&path) {
            *self = crate::load::parse_ini_str(&content);
        }
        mutator(self);

        let tmp_path = path.with_extension("tmp");
        let content = self.to_ini_full();
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path)?;

        // `atomic_update` is a second persistence path (locked read-modify-
        // write, used by enforcement.rs). It must mirror to the DB too —
        // otherwise a mutation taken via this path reaches the INI cache but
        // not the durable session_runtime row.
        //
        // The write-through runs UNDER the lock, before unlock: two racing
        // atomic_update calls would otherwise commit to the INI in one order
        // but to the DB in the other (lost update — the DB row goes stale vs
        // the file). Holding the lock across the RPC serializes DB writes in
        // file-commit order. Cost: ~1-3ms RPC under lock — acceptable on this
        // enforcement path (not the PreToolUse hot path).
        self.write_through_to_db(&content);
        lock_file.unlock()?;
        Ok(())
    }
}

pub(crate) fn join_csv(items: &[String]) -> String {
    items.join(",")
}
