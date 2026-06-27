use chrono::Local;
use crate::helpers::contains_any;
use crate::router::framework_detect::load_framework_patterns;
pub static FORBIDDEN_PHRASES: &[&str] = &[
    "Based on my knowledge",
    "I think",
    "I believe",
    "In my experience",
    "As I understand",
    "I recall",
    "From what I know",
];
#[derive(Debug)]
#[non_exhaustive]
pub struct ResearchGate {
    pub today_date: String,
    pub current_year: String,
}
impl ResearchGate {
    #[must_use]
    pub fn new() -> Self {
        let now = Local::now();
        Self {
            today_date: now.format("%Y-%m-%d").to_string(),
            current_year: now.format("%Y").to_string(),
        }
    }
    #[must_use]
    pub fn require_research(&self, task: &str) -> Option<ResearchRequirement> {
        let lower = task.to_lowercase();
        let fw = load_framework_patterns();
        for f in &fw {
            if lower.contains(f) {
                return Some(ResearchRequirement {
                    topic: f.clone(),
                    keywords: vec![format!("{f} {} best practices", self.current_year)],
                    mandatory: true,
                    reason: "Framework patterns change frequently - research current version"
                        .into(),
                });
            }
        }
        if contains_any(&lower, &["api", "syntax", "pattern", "implement"]) {
            return Some(ResearchRequirement {
                topic: "implementation".into(),
                keywords: vec![format!("current best practices {}", self.current_year)],
                mandatory: true,
                reason: "Implementation patterns evolve - verify current approach".into(),
            });
        }
        None
    }
    #[must_use]
    pub fn build_search_query(&self, topic: &str) -> String {
        format!("{topic} {} documentation latest", self.current_year)
    }
    #[must_use]
    pub fn validate_research_done(&self, context: &str) -> bool {
        let patterns = [
            "WebSearch",
            "WebFetch",
            "researched",
            "documentation shows",
            "according to",
            "latest docs",
        ];
        patterns.iter().any(|p| context.contains(p))
    }
    #[must_use]
    pub fn check_forbidden_phrases(&self, text: &str) -> Vec<String> {
        let lower = text.to_lowercase();
        FORBIDDEN_PHRASES
            .iter()
            .filter(|p| lower.contains(&p.to_lowercase()))
            .map(ToString::to_string)
            .collect()
    }
}
impl Default for ResearchGate {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResearchRequirement {
    pub topic: String,
    pub keywords: Vec<String>,
    pub mandatory: bool,
    pub reason: String,
}
#[cfg(test)]
#[path = "research_gate_tests.rs"]
mod tests;
