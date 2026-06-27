//! RPC-boundary witness gate: refuse a `roadmap` completion promotion that lacks
//! a valid, fresh witness receipt. Cheap (one `git rev-parse HEAD`, no cargo) so
//! it cannot block the async daemon. SOURCE: decision.cli-verifier.witness-receipt-rpc-boundary.

use kavach_patterns::witness_receipt::{Receipt, validate};

/// `Some(msg)` REFUSES the promotion (msg is agent-facing); `None` allows it.
/// Reads the live `HEAD` + wall clock, then delegates to the pure [`decide`].
pub(super) fn enforce_receipt(
    category: &str,
    status: &str,
    receipt: Option<&Receipt>,
) -> Option<String> {
    if !is_gated(category, status) {
        return None;
    }
    let head = git_head().unwrap_or_default();
    let now_ms = now_ms();
    // The promoting session must be the one in the receipt; here both anchors
    // come from the caller, so the load-bearing anti-replay teeth is head==HEAD
    // (the daemon reads HEAD itself — uncforgeable by the caller).
    let session_now = receipt.map_or("", |r| r.session_id.as_str());
    decide(category, status, receipt, &head, now_ms, session_now)
}

/// Pure gate decision — all live inputs injected so it is unit-testable.
fn decide(
    category: &str,
    status: &str,
    receipt: Option<&Receipt>,
    head_now: &str,
    now_ms: i64,
    session_now: &str,
) -> Option<String> {
    if !is_gated(category, status) {
        return None;
    }
    let Some(r) = receipt else {
        return Some(format!(
            "REFUSED: [{category}] -> {status} needs a witness receipt. Run the workspace \
             witness (cargo check+clippy+nextest) and resubmit with the receipt — a \
             completion claim must be backed by a passing build, not self-report."
        ));
    };
    match validate(r, head_now, now_ms, session_now) {
        Ok(()) => None,
        Err(e) => Some(format!(
            "REFUSED: [{category}] -> {status}: witness receipt invalid ({e}). Re-run the \
             workspace witness against the current HEAD and resubmit."
        )),
    }
}

/// True iff this is a `roadmap` completion status (`done`/`verified`).
fn is_gated(category: &str, status: &str) -> bool {
    category == "roadmap"
        && status
            .parse::<kavach_types::MemoryStatus>()
            .is_ok_and(kavach_types::MemoryStatus::is_complete)
}

/// `git rev-parse HEAD` in the daemon CWD, trimmed. `None` if git is absent or
/// the dir is not a repo — `enforce_receipt` then compares against "", which a
/// real receipt's non-empty `git_head` can never match → fail-closed REFUSE.
fn git_head() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// Epoch milliseconds, saturating. `0` on a pre-epoch clock — a real receipt's
/// `ts_ms` is then in the "future" and refused (fail-closed).
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
#[path = "witness_gate_test.rs"]
#[cfg(test)]
#[path = "witness_gate_test.rs"]
mod tests;