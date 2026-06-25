use std::path::Path;
use fs2::FileExt;

pub(super) const DEPLOY_LOCK_NAME: &str = ".deploy.lock";

/// RAII holder for OS-level `flock` (via `fs2`) serializing `kavach deploy` runs.
#[derive(Debug)]
pub(super) struct DeployLock {
    file: std::fs::File,
}

impl DeployLock {
    /// Try to acquire the workspace deploy lock without blocking (Ok(Some(_)) win, Ok(None) held).
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
