// SOURCE: https://docs.rs/dashmap/latest/dashmap/struct.DashMap.html (DashMap 6.1)
use dashmap::DashMap;

use super::types::RoutingDecision;

#[derive(Debug)]
pub struct SkillFirstRouter {
    skill_triggers: DashMap<String, String>,
    agent_skills: DashMap<String, Vec<String>>,
}

impl SkillFirstRouter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            skill_triggers: DashMap::new(),
            agent_skills: DashMap::new(),
        }
    }

    pub fn register_skill_trigger(&self, keyword: &str, skill_name: &str) {
        self.skill_triggers
            .insert(keyword.to_lowercase(), skill_name.into());
    }

    pub fn register_agent_skills(&self, agent_name: &str, skills: Vec<String>) {
        self.agent_skills.insert(agent_name.into(), skills);
    }

    #[must_use]
    pub fn route(&self, intent: &str, keywords: &[&str]) -> RoutingDecision {
        // ARCH: clone-on-get to release DashMap shard lock immediately
        // PATTERN: DashMap Ref retention deadlock prevention (per docs.rs/dashmap)
        // SCOPE: per-call (no cross-call state)
        // FAILURE: holding a Ref across .insert/.get on same map = deadlock
        // SOURCE: https://docs.rs/dashmap/latest/dashmap/struct.DashMap.html#deadlock
        for kw in keywords {
            let skill = self
                .skill_triggers
                .get(&kw.to_lowercase())
                .map(|r| r.value().clone());
            if let Some(skill_name) = skill {
                return RoutingDecision {
                    use_skill: true,
                    skill_name,
                    agent_name: String::new(),
                    requires_ceo: false,
                    reason: "Keyword trigger matched skill".into(),
                };
            }
        }
        let lower = intent.to_lowercase();
        let mappings = kavach_config::get_intent_skill_mappings();
        for (keyword, skill) in &mappings {
            if lower.contains(keyword) {
                return RoutingDecision {
                    use_skill: true,
                    skill_name: skill.clone(),
                    agent_name: String::new(),
                    requires_ceo: false,
                    reason: "Intent matched skill domain".into(),
                };
            }
        }
        let complex = kavach_config::get_complex_indicators();
        for indicator in &complex {
            if lower.contains(indicator) {
                return RoutingDecision {
                    use_skill: false,
                    skill_name: String::new(),
                    agent_name: "ceo".into(),
                    requires_ceo: true,
                    reason: "Complex task requires orchestration".into(),
                };
            }
        }
        RoutingDecision {
            use_skill: false,
            skill_name: String::new(),
            agent_name: "backend-engineer".into(),
            requires_ceo: false,
            reason: "No specific skill match, using default agent".into(),
        }
    }

    #[must_use]
    pub fn get_skill_for_agent(&self, agent_name: &str) -> String {
        // Clone first element out before any further map access — avoid ref retention.
        let first_skill = self
            .agent_skills
            .get(agent_name)
            .and_then(|r| r.value().first().cloned());
        if let Some(s) = first_skill {
            return s;
        }
        let defaults = kavach_config::get_skill_agent_defaults();
        defaults.get(agent_name).cloned().unwrap_or_default()
    }

    #[must_use]
    pub fn should_prefer_skill(&self, task_type: &str) -> bool {
        let prefs = kavach_config::get_skill_preferred_keywords();
        let lower = task_type.to_lowercase();
        prefs.iter().any(|p| lower.contains(p))
    }
}

impl Default for SkillFirstRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "skill_first_tests.rs"]
mod tests;
