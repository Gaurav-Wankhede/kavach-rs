//! TDD: run record/update-status CLI param builders. Pure JSON builders are the
//! unit-tested core; the RPC round-trip runs against the live daemon.
use super::*;

#[test]
fn record_params_carry_required_and_optional() {
    let v = build_record_params("kavach-rs", "roadmap.unit.x", Some("main"), "running", Some(42));
    assert_eq!(v["project"], "kavach-rs");
    assert_eq!(v["entry_key"], "roadmap.unit.x");
    assert_eq!(v["branch"], "main");
    assert_eq!(v["status"], "running");
    assert_eq!(v["pid"], 42);
}

#[test]
fn record_params_omit_none_as_null() {
    let v = build_record_params("p", "k", None, "done", None);
    assert!(v["branch"].is_null());
    assert!(v["pid"].is_null());
}

#[test]
fn update_status_params_carry_id_and_status() {
    let v = build_update_params("run:abc", "done", Some(0));
    assert_eq!(v["id"], "run:abc");
    assert_eq!(v["status"], "done");
    assert_eq!(v["exit_code"], 0);
}
