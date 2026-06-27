use std::collections::HashMap;
// SOURCE: https://docs.rs/linfa-trees/ — decision tree classification integration
use crate::chain_state::ChainState;
use crate::helpers::{contains_any, extract_agents};
use crate::intent_features::extract_features;
use crate::intent_tree::build_intent_tree;
use crate::types::{IntentAnalysis, VerificationResult};
pub(crate) fn run_gate(state: &mut ChainState, prompt: &str) {
    let intent = analyze_intent(prompt);
    let mut result = VerificationResult {
        gate: "INTENT".into(),
        status: "pass".into(),
        reason: format!(
            "type={} confidence={:.2} risk={}",
            intent.intent_type, intent.confidence, intent.risk_level
        ),
        context: HashMap::from([
            ("type".into(), intent.intent_type.clone()),
            ("complexity".into(), intent.complexity.clone()),
            ("risk_level".into(), intent.risk_level.clone()),
        ]),
        timestamp: String::new(),
        next_action: String::new(),
    };
    if intent.risk_level == "critical" && intent.confidence < 0.7 {
        result.status = "warn".into();
        result.reason = format!(
            "INTENT: Critical risk with low confidence ({:.2}) — verify intent before proceeding",
            intent.confidence
        );
    }
    // [ROUTE] one-line agent suggestion per CLAUDE.md §13 budget.
    // 3-tier cost-balance suppression (decision:rca.agent_routing_token_cost):
    //   T1: skip when complexity=simple — trivial work doesn't justify suggestion overhead
    //   T2: skip when complexity=moderate AND no agent has capability match (score<100)
    //       — description-only matches are low-confidence; not worth the tokens
    //   T3: per-session repeat-suppression — if same agent suggested ≥3 times AND
    //       not yet delegated, drop it from future suggestions this session
    // T1: suppress when complexity=simple AND risk=low (trivial work, no agent needed).
    // Otherwise allow the suggestion path to evaluate (T2/T3 may still drop it).
    let is_trivial = matches!(intent.complexity.as_str(), "simple")
        && matches!(intent.risk_level.as_str(), "low");
    let should_suggest = !is_trivial;
    if should_suggest && let Some(loader) = crate::loader::global_loader() {
        let raw = loader.suggest_for_intent(&intent.intent_type, prompt, 3);
        // T2: require at least one strong capability match (score ≥ 100) for
        // moderate-complexity prompts; complex/critical bypass this filter.
        let has_capability_match = raw.iter().any(|(_, s)| *s >= 100);
        let pass_t2 = matches!(intent.complexity.as_str(), "complex")
            || intent.risk_level == "critical"
            || has_capability_match;
        if pass_t2 {
            // T3: drop saturated agents (already suggested ≥3× this session).
            let filtered: Vec<_> = raw
                .into_iter()
                .filter(|(a, _)| !state.is_suggestion_saturated(&a.name, 3))
                .collect();
            if !filtered.is_empty() {
                let names: Vec<String> = filtered
                    .iter()
                    .map(|(a, s)| format!("{}({s})", a.name))
                    .collect();
                result
                    .context
                    .insert("route_suggestions".into(), names.join(","));
                // Record each suggestion. T3 filter above prevents >3
                // suggestions per agent per session: counts 1, 2, 3 pass;
                // count=3 returns true from is_suggestion_saturated, so the
                // 4th attempt is dropped before reaching this line.
                for (a, _) in &filtered {
                    state.record_suggestion(&a.name);
                }
            }
        }
    }
    state.intent = Some(intent);
    state.add_result(result);
}
#[must_use]
pub fn analyze_intent(prompt: &str) -> IntentAnalysis {
    // Primary path: decision tree classification
    let tree = build_intent_tree();
    let features = extract_features(prompt);
    if let Ok(outcome) = tree.classify(&features) {
        let lower = prompt.to_lowercase();
        let sd = kavach_config::paths::skills_dir();
        let mut skills = outcome.required_skills.clone();
        // Augment with NLP keyword routing for additional skills
        for skill in kavach_patterns::skill_keyword_router::skills_from_keywords(prompt) {
            if sd.join(&skill).join("SKILL.md").exists() && !skills.iter().any(|s| s == &skill) {
                skills.push(skill);
            }
        }
        return IntentAnalysis {
            intent_type: outcome.intent_type.clone(),
            confidence: outcome.confidence,
            required_skills: skills,
            required_agents: extract_agents(&lower),
            // Single source of truth: the dtree leaf's hardcoded bool may only
            // RAISE the canonical config-driven decision, never contradict it.
            // (Was `outcome.requires_research` alone → disagreed with the config
            // + research_guard paths on the same prompt = TABULA_RASA misfire.)
            requires_research: outcome.requires_research
                || kavach_config::requires_research(prompt),
            complexity: outcome.complexity.clone(),
            risk_level: outcome.risk_level.clone(),
        };
    }
    // Fallback: keyword-based classification
    classify_by_keywords(prompt)
}
/// Fallback intent classifier used when the decision tree abstains.
/// Pure keyword/NLP routing — extracted from `analyze_intent` to keep each
/// function under the `too_many_lines` threshold.
fn classify_by_keywords(prompt: &str) -> IntentAnalysis {
    let lower = prompt.to_lowercase();
    let mut a = IntentAnalysis {
        intent_type: "general".into(),
        confidence: 0.5,
        required_skills: Vec::new(),
        required_agents: Vec::new(),
        requires_research: true,
        complexity: "simple".into(),
        risk_level: "low".into(),
    };
    apply_keyword_arms(&lower, &mut a);
    augment_skills_from_keywords(prompt, &mut a.required_skills);
    a.required_agents = extract_agents(&lower);
    a
}
/// Applies the static keyword-category arms to a working `IntentAnalysis`.
/// Each arm refines intent type, risk, complexity, confidence, and skills.
fn apply_keyword_arms(lower: &str, a: &mut IntentAnalysis) {
    if contains_any(
        lower,
        &["implement", "create", "build", "add", "develop", "write"],
    ) {
        a.intent_type = "implement".into();
        a.requires_research = true;
        a.complexity = "moderate".into();
        a.confidence = 0.8;
    }
    if contains_any(
        lower,
        &[
            "fix",
            "bug",
            "error",
            "debug",
            "broken",
            "not working",
            "crash",
            "find",
            "discover",
            "locate",
            "trace",
            "investigate",
            "diagnose",
            "troubleshoot",
            "worst",
        ],
    ) {
        a.intent_type = "debug".into();
        if kavach_config::paths::skills_dir()
            .join("debug-like-expert")
            .join("SKILL.md")
            .exists()
        {
            a.required_skills.push("debug-like-expert".into());
        }
        a.complexity = "moderate".into();
        a.confidence = 0.85;
    }
    if contains_any(
        lower,
        &["refactor", "restructure", "clean up", "improve", "optimize"],
    ) {
        a.intent_type = "refactor".into();
        a.requires_research = true;
        a.complexity = "complex".into();
        a.risk_level = "medium".into();
        a.confidence = 0.8;
    }
    if contains_any(
        lower,
        &["deploy", "release", "publish", "production", "go live"],
    ) {
        a.intent_type = "deploy".into();
        a.required_skills
            .push("cloud-infrastructure-mastery".into());
        a.risk_level = "high".into();
        a.complexity = "complex".into();
        a.confidence = 0.9;
        a.requires_research = true;
    }
    if contains_any(lower, &["security", "auth", "encrypt", "vulnerability"]) {
        a.intent_type = "security".into();
        a.required_skills.push("security".into());
        a.risk_level = "high".into();
        a.requires_research = true;
        a.confidence = 0.85;
    }
    if contains_any(
        lower,
        &[
            "memory bank",
            "update memory",
            "remember this",
            "save to memory",
        ],
    ) {
        a.intent_type = "memory".into();
        a.confidence = 0.9;
        a.complexity = "simple".into();
        a.requires_research = false;
    }
    if contains_any(lower, &["delete", "remove", "drop", "destroy", "purge"]) {
        a.risk_level = "critical".into();
        a.complexity = "complex".into();
        a.confidence = 0.75;
    }
}
/// Dynamic NLP skill routing — Aho-Corasick multi-pattern matching across all
/// installed skills. Appends only skills that exist on disk and are not already
/// present, deduplicating against `skills`.
fn augment_skills_from_keywords(prompt: &str, skills: &mut Vec<String>) {
    let sd = kavach_config::paths::skills_dir();
    for skill in kavach_patterns::skill_keyword_router::skills_from_keywords(prompt) {
        if sd.join(&skill).join("SKILL.md").exists() && !skills.iter().any(|s| s == &skill) {
            skills.push(skill);
        }
    }
}
#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
