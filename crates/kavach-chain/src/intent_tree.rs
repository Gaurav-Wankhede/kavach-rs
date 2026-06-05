// SOURCE: https://docs.rs/linfa-trees/ — decision tree classification pattern
// SOURCE: https://github.com/Entscheider/stamm — generic decision trees for Rust

use kavach_dtree::{DecisionNode, DecisionTree, Outcome, Predicate};

/// Build the default intent classification tree.
/// Tree structure:
///   - Check destructive keywords → critical risk
///   - Check deploy keywords → high risk, ops skill
///   - Check security keywords → high risk, security skill
///   - Check debug keywords → moderate, debug skill
///   - Check refactor keywords → medium risk
///   - Check implement keywords → moderate, research required
///   - Default → general intent
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "linear dispatcher building decision tree bottom-up; refactoring would reduce clarity"
)]
pub fn build_intent_tree() -> DecisionTree {
    let critical_leaf = DecisionNode::leaf(Outcome {
        intent_type: "destructive".into(),
        complexity: "complex".into(),
        risk_level: "critical".into(),
        required_skills: Vec::new(),
        requires_research: true,
        confidence: 0.75,
    });

    let deploy_leaf = DecisionNode::leaf(Outcome {
        intent_type: "deploy".into(),
        complexity: "complex".into(),
        risk_level: "high".into(),
        required_skills: vec!["ops".into()],
        requires_research: true,
        confidence: 0.9,
    });

    let security_leaf = DecisionNode::leaf(Outcome {
        intent_type: "security".into(),
        complexity: "complex".into(),
        risk_level: "high".into(),
        required_skills: vec!["rust".into()],
        requires_research: true,
        confidence: 0.85,
    });

    let debug_leaf = DecisionNode::leaf(Outcome {
        intent_type: "debug".into(),
        complexity: "moderate".into(),
        risk_level: "medium".into(),
        required_skills: vec!["bug-bounty".into()],
        requires_research: true,
        confidence: 0.85,
    });

    let refactor_leaf = DecisionNode::leaf(Outcome {
        intent_type: "refactor".into(),
        complexity: "complex".into(),
        risk_level: "medium".into(),
        required_skills: Vec::new(),
        requires_research: true,
        confidence: 0.8,
    });

    let implement_leaf = DecisionNode::leaf(Outcome {
        intent_type: "implement".into(),
        complexity: "moderate".into(),
        risk_level: "low".into(),
        required_skills: Vec::new(),
        requires_research: true,
        confidence: 0.8,
    });

    let memory_leaf = DecisionNode::leaf(Outcome {
        intent_type: "memory".into(),
        complexity: "simple".into(),
        risk_level: "low".into(),
        required_skills: Vec::new(),
        requires_research: false,
        confidence: 0.9,
    });

    let general_leaf = DecisionNode::leaf(Outcome::default());

    // Build tree bottom-up — memory checked early (before debug) since it's specific
    let implement_branch = DecisionNode::branch(
        Predicate::BooleanIs {
            name: "has_implement".into(),
            expected: true,
        },
        implement_leaf,
        general_leaf,
    );

    let refactor_branch = DecisionNode::branch(
        Predicate::BooleanIs {
            name: "has_refactor".into(),
            expected: true,
        },
        refactor_leaf,
        implement_branch,
    );

    let debug_branch = DecisionNode::branch(
        Predicate::BooleanIs {
            name: "has_debug".into(),
            expected: true,
        },
        debug_leaf,
        refactor_branch,
    );

    // Memory checked BEFORE debug — "save to memory" contains "find" which triggers debug
    let memory_branch = DecisionNode::branch(
        Predicate::BooleanIs {
            name: "has_memory".into(),
            expected: true,
        },
        memory_leaf,
        debug_branch,
    );

    let security_branch = DecisionNode::branch(
        Predicate::BooleanIs {
            name: "has_security".into(),
            expected: true,
        },
        security_leaf,
        memory_branch,
    );

    let deploy_branch = DecisionNode::branch(
        Predicate::BooleanIs {
            name: "has_deploy".into(),
            expected: true,
        },
        deploy_leaf,
        security_branch,
    );

    let root = DecisionNode::branch(
        Predicate::BooleanIs {
            name: "has_destructive".into(),
            expected: true,
        },
        critical_leaf,
        deploy_branch,
    );

    DecisionTree::new("intent-classifier-v1", root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kavach_dtree::FeatureSet;

    #[test]
    fn classifies_destructive_as_critical() {
        let tree = build_intent_tree();
        let features = FeatureSet::new()
            .with_bool("has_destructive", true)
            .with_bool("has_deploy", false)
            .with_bool("has_security", false)
            .with_bool("has_memory", false)
            .with_bool("has_debug", false)
            .with_bool("has_refactor", false)
            .with_bool("has_implement", false);

        let outcome = tree.classify(&features).expect("classification failed");
        assert_eq!(outcome.risk_level, "critical");
        assert_eq!(outcome.intent_type, "destructive");
    }

    #[test]
    fn classifies_debug_correctly() {
        let tree = build_intent_tree();
        let features = FeatureSet::new()
            .with_bool("has_destructive", false)
            .with_bool("has_deploy", false)
            .with_bool("has_security", false)
            .with_bool("has_memory", false)
            .with_bool("has_debug", true)
            .with_bool("has_refactor", false)
            .with_bool("has_implement", false);

        let outcome = tree.classify(&features).expect("classification failed");
        assert_eq!(outcome.intent_type, "debug");
        assert!(outcome.required_skills.contains(&"bug-bounty".to_owned()));
    }

    #[test]
    fn defaults_to_general() {
        let tree = build_intent_tree();
        let features = FeatureSet::new()
            .with_bool("has_destructive", false)
            .with_bool("has_deploy", false)
            .with_bool("has_security", false)
            .with_bool("has_debug", false)
            .with_bool("has_refactor", false)
            .with_bool("has_implement", false)
            .with_bool("has_memory", false);

        let outcome = tree.classify(&features).expect("classification failed");
        assert_eq!(outcome.intent_type, "general");
    }
}
