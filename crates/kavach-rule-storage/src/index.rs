//! Fast lookup index for rules by trigger keyword and category.

use std::collections::HashMap;

use crate::store::StoredRule;

#[derive(Debug, Default)]
pub struct RuleIndex {
    by_trigger: HashMap<String, Vec<String>>,
    by_category: HashMap<String, Vec<String>>,
}

impl RuleIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the entire index from the current rule cache.
    pub fn rebuild(&mut self, rules: &HashMap<String, StoredRule>) {
        self.by_trigger.clear();
        self.by_category.clear();
        for (name, rule) in rules {
            for trigger in &rule.definition.metadata.triggers {
                self.by_trigger
                    .entry(trigger.to_lowercase())
                    .or_default()
                    .push(name.clone());
            }
            let cat = extract_category(&rule.definition.metadata.protocol);
            self.by_category.entry(cat).or_default().push(name.clone());
        }
    }

    /// Get rule names matching a trigger keyword (case-insensitive).
    #[must_use]
    pub fn by_trigger(&self, trigger: &str) -> Vec<&str> {
        self.by_trigger
            .get(&trigger.to_lowercase())
            .map(|v| v.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Get rule names matching a category string.
    #[must_use]
    pub fn by_category(&self, category: &str) -> Vec<&str> {
        self.by_category
            .get(&category.to_lowercase())
            .map(|v| v.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

}

fn extract_category(protocol: &str) -> String {
    protocol
        .split('/')
        .next()
        .unwrap_or("unknown")
        .to_lowercase()
}
