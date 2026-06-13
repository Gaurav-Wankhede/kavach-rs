use super::*;

#[tokio::test]
async fn test_open_memory() -> Result<()> {
    let db = open_memory().await?;
    let info: Option<serde_json::Value> = db.query("INFO FOR DB").await?.take(0)?;
    info.ok_or_else(|| Error::RecordNotFound("INFO FOR DB returned empty result".to_owned()))?;
    Ok(())
}

/// The OS LOCK-holder probe must never return THIS process and must tolerate a
/// missing/unheld lock (no panic, no false positive). On an unheld default path
/// (no daemon in a test sandbox) it returns None or a foreign live PID — never
/// our own pid. Guards the orphaned-holder fallback that fixes the wedged stop
/// hook (rca.stop-hook-surreal-lock-orphaned-holder).
#[cfg(unix)]
#[test]
fn lock_holder_probe_never_targets_self_and_tolerates_unheld() {
    let found = lock_holder_pid_via_os();
    let self_pid = i32::try_from(std::process::id()).unwrap_or(-1);
    assert_ne!(found, Some(self_pid), "must never SIGTERM the calling process");
    // Whatever it returns must be a positive, non-self PID (or None).
    if let Some(pid) = found {
        assert!(pid > 0, "PID must be positive (never a pgid/0): {pid}");
    }
}
