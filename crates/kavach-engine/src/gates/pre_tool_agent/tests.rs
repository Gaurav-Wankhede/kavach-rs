//! Contract registry lookups + read-only flag correctness.
use super::get_contract;

#[test]
fn known_agents_have_contracts() {
    assert!(get_contract("research-director").is_some());
    assert!(get_contract("backend-engineer").is_some());
    assert!(get_contract("unknown").is_none());
}

#[test]
fn read_only_flag_correct() {
    assert!(get_contract("research-director").unwrap().read_only);
    assert!(!get_contract("backend-engineer").unwrap().read_only);
}
