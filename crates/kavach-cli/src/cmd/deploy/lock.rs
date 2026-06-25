use std::path::Path;
use fs2::FileExt;

pub(super) const DEPLOY_LOCK_NAME: &str = ".deploy.lock";

/// RAII holder for the exclusive advisory `flock` that serializes concurrent
/// `kavach deploy` runs. The lock is an OS-level `fcntl`/`flock` (via `fs2`), so
/// the kernel releases it automatically if the process dies mid-deploy — a
/// crashed run can never leave a stale lock that wedges the next deploy (unlike
/// a manually-managed sentinel file). The file itself is intentionally NOT
/// removed on drop: keeping it lets the next run re-lock the same inode, and an
/// empty leftover `.deploy.lock` is harmless. Dropping the handle unlocks.
#[derive(Debug)]
pub(super) struct DeployLock {
    file: std::fs::File,
}

impl DeployLock {
    /// Try to acquire the workspace deploy lock without blocking.
    ///
    /// Returns `Ok(Some(guard))` when this process won the lock, `Ok(None)` when
    /// another `kavach deploy` already holds it (the caller must refuse to
    /// proceed — two concurrent installs race the binary copy + daemon restart),
    /// and `Err` only on an unexpected filesystem error opening the lock file.
    pub(super) fn try_acquire(root: &Path) -> std::io::Result<Option<Self>> {
        let path = root.join(DEPLOY_LOCK_NAME);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { file })),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl Drop for DeployLock {
    fn drop(&mut self) {
        drop(FileExt::unlock(&self.file));
    }
}
