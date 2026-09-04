// Unit tests for bulk_manifest::is_usable purity. DB ops are exercised by
// the integration suite (tests/bulk_manifest_smoke.rs) once the schema lands.
use super::types::{BulkManifest, STATUS_ACTIVE, STATUS_CLOSED, STATUS_EXPIRED, is_usable};
use chrono::{Duration, Utc};

fn fixture(status: &str, expires_in_secs: i64) -> BulkManifest {
    let now = Utc::now();
    BulkManifest {
        id: None,
        sweep_id: "bulk.test-1".to_owned(),
        project: "kavach-rs".to_owned(),
        root_rca: "[RCA]...".to_owned(),
        scope_glob: "crates/**/*.rs".to_owned(),
        lint_class: "indexing_slicing".to_owned(),
        fix_strategy: "a[i] -> a.get(i)?".to_owned(),
        blast_estimate: 100,
        signed_by_session: "sess_x".to_owned(),
        approved_by: "user".to_owned(),
        approved_at: now,
        expires_at: now
            .checked_add_signed(Duration::seconds(expires_in_secs))
            .unwrap_or(now),
        conformance_applied: 0,
        conformance_refused: 0,
        conformance_drifted: 0,
        status: status.to_owned(),
        closed_at: None,
    }
}

#[test]
fn is_usable_when_active_and_not_expired() {
    let m = fixture(STATUS_ACTIVE, 3600);
    assert!(is_usable(&m, Utc::now()));
}

#[test]
fn is_usable_false_when_expired_by_clock() {
    let m = fixture(STATUS_ACTIVE, -1);
    assert!(!is_usable(&m, Utc::now()));
}

#[test]
fn is_usable_false_when_closed_even_with_time_remaining() {
    let m = fixture(STATUS_CLOSED, 3600);
    assert!(!is_usable(&m, Utc::now()));
}

#[test]
fn is_usable_false_when_expired_status() {
    let m = fixture(STATUS_EXPIRED, 3600);
    assert!(!is_usable(&m, Utc::now()));
}
