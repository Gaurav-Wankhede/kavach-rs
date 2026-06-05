// SOURCE: https://docs.rs/linfa-trees/ — decision-tree classification pattern
// SOURCE: https://github.com/Entscheider/stamm — generic decision trees for Rust
// SOURCE: arxiv 2603.04582 Self-Attribution-Bias (2026) — the stop verdict
//   must be a deterministic TREE PATH over observable features, never the
//   model's self-judgement. This tree is the decision layer over
//   `stop_features::extract_stop_features`; the verdict is the leaf reached,
//   NOT a literal phrase hit.
//
// WHY a tree, not an `if` ladder: the four stop-classes (PLAN_STALL /
// SCOPE_ASK / DEFER / CLEAN) share overlapping features (a plan-stall and a
// scope-ask BOTH mention a plan; a scope-ask and a defer BOTH lack code).
// Encoding the precedence as a `DecisionTree` (the same `kavach-dtree`
// substrate `intent_tree.rs` already uses) makes the resolution order
// explicit and serializable, and removes the hand-tangled boolean soup that
// let the original stop_detect.rs detectors be reordered/escaped.
//
// PRECEDENCE (root → leaves), highest first:
//   1. impl_evidence ∨ had_write_this_turn   → CLEAN  (code shipped; a turn
//        that planned AND coded is never a stall — §EVIDENCE artifact wins)
//   2. authored_artifact ∧ resume_elsewhere  → PLAN_STALL  (wrote a plan/
//        spec/roadmap then punted execution elsewhere, no code — THE bug)
//   3. strong_scope_ask                      → SCOPE_ASK  (genuine §FOCUS-
//        sanctioned user-directed question — must NOT be punished)
//   4. resume_elsewhere (alone)              → DEFER  (generic "later/next
//        session" with no artifact and no ask — ordinary deferral)
//   5. else                                  → CLEAN

use kavach_dtree::{DecisionNode, DecisionTree, Outcome, Predicate};

/// Stop-message verdict.
///
/// `intent_type` is the discriminant the engine switches on; the other
/// `Outcome` fields are populated for parity with `intent_tree.rs` consumers
/// (risk/skills unused by the stop path).
pub const PLAN_STALL: &str = "plan_stall";
pub const SCOPE_ASK: &str = "scope_ask";
pub const DEFER: &str = "defer";
pub const CLEAN: &str = "clean";

fn leaf(intent: &str, confidence: f64) -> DecisionNode {
    DecisionNode::leaf(Outcome {
        intent_type: intent.into(),
        complexity: "simple".into(),
        risk_level: if intent == CLEAN {
            "low".into()
        } else {
            "medium".into()
        },
        required_skills: Vec::new(),
        requires_research: false,
        confidence,
    })
}

fn is(name: &str, expected: bool) -> Predicate {
    Predicate::BooleanIs {
        name: name.into(),
        expected,
    }
}

/// Build the stop-intent decision tree. Evaluated top-down; the FIRST
/// satisfied branch wins, which is exactly the precedence list above.
#[must_use]
pub fn build_stop_intent_tree() -> DecisionTree {
    // Node 4: resume_elsewhere alone → DEFER, else CLEAN.
    let defer_or_clean = DecisionNode::branch(
        is("resume_elsewhere", true),
        leaf(DEFER, 0.80),
        leaf(CLEAN, 0.85),
    );

    // Node 3: a genuine scope ask → SCOPE_ASK, else fall to node 4.
    let scope_or_below = DecisionNode::branch(
        is("strong_scope_ask", true),
        leaf(SCOPE_ASK, 0.85),
        defer_or_clean,
    );

    // Node 2: authored_artifact ∧ resume_elsewhere → PLAN_STALL, else node 3.
    let stall_or_below = DecisionNode::branch(
        Predicate::And(
            Box::new(is("authored_artifact", true)),
            Box::new(is("resume_elsewhere", true)),
        ),
        leaf(PLAN_STALL, 0.90),
        scope_or_below,
    );

    // Node 1 (root): code shipped this turn → CLEAN, else node 2.
    // `impl_evidence ∨ had_write_this_turn` — the §EVIDENCE artifact
    // predicate dominates everything: a turn that produced a verified build
    // or a real file mutation is never a stall, regardless of prose.
    let root = DecisionNode::branch(
        Predicate::Or(
            Box::new(is("impl_evidence", true)),
            Box::new(is("had_write_this_turn", true)),
        ),
        leaf(CLEAN, 0.95),
        stall_or_below,
    );

    DecisionTree::new("stop_intent", root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stop_features::extract_stop_features;

    // Drive the tree end-to-end from raw text via the regex feature layer —
    // this is the real integration the engine performs.
    fn verdict(msg: &str, wrote: bool) -> String {
        let feats = extract_stop_features(msg, wrote).expect("patterns compile");
        let tree = build_stop_intent_tree();
        tree.classify(&feats)
            .expect("tree is total — every feature path reaches a leaf")
            .intent_type
            .clone()
    }

    #[test]
    fn nicole_carpenter_transcript_is_plan_stall() {
        // The exact lazy closing the user flagged: plan written, "fresh
        // session", build "proceeds immediately", ZERO code this turn.
        let msg = "The §PLAN is written. Start a fresh session pointed at \
                   that plan file and the build proceeds immediately.";
        assert_eq!(verdict(msg, false), PLAN_STALL);
    }

    #[test]
    fn planned_then_coded_is_clean_not_stall() {
        // Same plan language BUT code shipped → artifact predicate wins.
        let msg = "Wrote the plan, then implemented it: cargo check passed, \
                   wired into the stop chain.";
        assert_eq!(verdict(msg, true), CLEAN);
    }

    #[test]
    fn genuine_scope_ask_is_not_punished() {
        let msg = "Which one do you want me to take? The existing helper \
                   or a new extractor — your call.";
        assert_eq!(verdict(msg, false), SCOPE_ASK);
    }

    #[test]
    fn generic_later_without_artifact_is_defer() {
        let msg = "I'll pick this up in the next session.";
        assert_eq!(verdict(msg, false), DEFER);
    }

    #[test]
    fn substantive_work_report_is_clean() {
        let msg = "Fixed the off-by-one in the parser; regression tests pass.";
        assert_eq!(verdict(msg, true), CLEAN);
    }

    #[test]
    fn plan_stall_beats_scope_ask_when_both_present() {
        // A plan-stall dressed up with a token question must still be caught
        // — this is the precise self-justifying-escape the user reported.
        let msg = "The §PLAN is written and execution-ready; the build \
                   proceeds in a dedicated session. Your call?";
        assert_eq!(verdict(msg, false), PLAN_STALL);
    }
}
