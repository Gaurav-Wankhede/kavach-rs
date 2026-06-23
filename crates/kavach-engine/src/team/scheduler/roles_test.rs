//! TDD: TRINITY role classification + role→backend routing over the vendor pool.
//! SOURCE: decision.fugu-orchestration-layer · https://sakana.ai/trinity/
use super::*;
use crate::team::AgentRole;

fn node(title: &str) -> DagNode {
    node_id(title, title)
}

fn node_id(id: &str, title: &str) -> DagNode {
    DagNode {
        id: id.into(),
        entry_key: id.into(),
        title: title.into(),
        entry_status: "todo".into(),
        category: "roadmap".into(),
    }
}

#[test]
fn plan_and_design_titles_are_thinker() {
    assert_eq!(role_for_title("[PLAN] decompose feature"), AgentRole::Thinker);
    assert_eq!(role_for_title("Author spec for X"), AgentRole::Thinker);
    assert_eq!(role_for_title("design the DAG scheduler"), AgentRole::Thinker);
}

#[test]
fn verify_titles_are_verifier() {
    assert_eq!(role_for_title("3-witness verify build + nextest"), AgentRole::Verifier);
    assert_eq!(role_for_title("Verify roadmap entry"), AgentRole::Verifier);
}

#[test]
fn default_title_is_worker() {
    assert_eq!(role_for_title("Implement the handler"), AgentRole::Worker);
    assert_eq!(role_for_title("split file to nano-files"), AgentRole::Worker);
}

#[test]
fn role_for_node_reads_title() {
    assert_eq!(role_for_node(&node("[PLAN] x")), AgentRole::Thinker);
    assert_eq!(role_for_node(&node("ship the route")), AgentRole::Worker);
}

#[test]
fn pool_routes_each_role_to_its_backend() {
    let pool = RolePool::default();
    // Thinker -> high-capability cc; Worker -> cost-efficient codex; Verifier -> in-house gates (cc).
    assert_eq!(pool.backend_for(AgentRole::Thinker).id(), "cc");
    assert_eq!(pool.backend_for(AgentRole::Worker).id(), "codex");
    assert_eq!(pool.backend_for(AgentRole::Verifier).id(), "cc");
}

#[test]
fn assignments_pair_each_key_with_its_role() {
    let dag = RoadmapDag {
        nodes: vec![node("[PLAN] a"), node("implement b")],
        ..Default::default()
    };
    let got = role_assignments(&["[PLAN] a".into(), "implement b".into()], &dag);
    assert_eq!(got[0].1, AgentRole::Thinker);
    assert_eq!(got[1].1, AgentRole::Worker);
}
