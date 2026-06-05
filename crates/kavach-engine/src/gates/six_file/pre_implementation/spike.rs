//! Spike-mode bypass: an unexpired `workflow.spike.active` row suspends the
//! spec gates and emits an advisory instead.

/// Return the spike reason iff a `workflow.spike.active` row exists and has not
/// expired (`expires_at_unix_s` in the future), else `None`.
pub(super) fn active_spike(rows: &[(String, String)]) -> Option<String> {
    let row = rows.iter().find(|(k, _)| k == "workflow.spike.active")?;
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
    .unwrap_or(i64::MAX);
    let mut expires_at = 0i64;
    let mut reason = String::new();
    for line in row.1.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("expires_at_unix_s=") {
            expires_at = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("reason=") {
            rest.trim().clone_into(&mut reason);
        }
    }
    (now < expires_at).then_some(reason)
}

/// Emit the spike-active advisory to the hook log channel (stderr).
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(super) fn emit_spike_advisory(reason: &str) {
    eprintln!(
        "[SPIKE_MODE_ACTIVE]\nSpike mode is active (reason: {reason}). Spec gates are bypassed."
    );
}
