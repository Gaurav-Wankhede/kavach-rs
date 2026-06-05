use crate::chain_state::ChainState;
use crate::types::VerificationResult;
use std::collections::HashMap;

#[test]
fn test_chain_state_block() {
    let mut cs = ChainState::new("test");
    cs.add_result(VerificationResult {
        gate: "AEGIS".into(),
        status: "block".into(),
        reason: "dangerous".into(),
        context: HashMap::new(),
        timestamp: String::new(),
        next_action: String::new(),
    });
    assert!(cs.is_blocked());
    assert!(cs.get_block_reason().contains("AEGIS"));
}
