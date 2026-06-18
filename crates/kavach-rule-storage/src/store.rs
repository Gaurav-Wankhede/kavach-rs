//! Central rule store: in-memory cache backed by disk persistence.

use std::collections::HashMap;
use std::path::PathBuf;

use kavach_rule_ast::SkillDefinition;
use serde::{Deserialize, Serialize};

use crate::error::{Result, StorageError};
use crate::index::RuleIndex;
use crate::version::RuleVersion;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredRule {
    pub definition: SkillDefinition,
    pub source_path: PathBuf,
    pub content_hash: String,
    pub last_modified: String,
    pub version: u32,
}

impl StoredRule {
    #[must_use]
    pub const fn new(
        definition: SkillDefinition,
        source_path: PathBuf,
        content_hash: String,
        last_modified: String,
        version: u32,
    ) -> Self {
        Self {
            definition,
            source_path,
            content_hash,
            last_modified,
            version,
        }
    }
}

// TIME: O(1) avg | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
#[derive(Debug)]
pub struct RuleStore {
    pub(crate) rules_dir: PathBuf,
    pub(crate) cache: HashMap<String, StoredRule>,
    pub(crate) index: RuleIndex,
}

impl RuleStore {
    #[must_use]
    pub fn new(rules_dir: PathBuf) -> Self {
        Self {
            rules_dir,
            cache: HashMap::new(),
            index: RuleIndex::new(),
        }
    }

    /// Load all rules from the rules directory.
    ///
    /// # Errors
    /// Returns [`StorageError`] if rules cannot be read or parsed.
    pub fn load_all(&mut self) -> Result<()> {
        self.cache.clear();
        let loaded = crate::loader::load_rules_from_dir(&self.rules_dir)?;
        for rule in loaded {
            let name = rule.definition.metadata.name.clone();
            self.cache.insert(name, rule);
        }
        self.index.rebuild(&self.cache);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&StoredRule> {
        self.cache.get(name)
    }

    /// Save a rule to disk and cache it.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the rule cannot be written.
    pub fn save(&mut self, rule: &StoredRule) -> Result<PathBuf> {
        let path = crate::writer::write_rule(&self.rules_dir, rule)?;
        let name = rule.definition.metadata.name.clone();
        let mut saved = rule.clone();
        saved.source_path.clone_from(&path);
        self.cache.insert(name, saved);
        self.index.rebuild(&self.cache);
        Ok(path)
    }

    #[must_use]
    pub fn list(&self) -> Vec<&str> {
        self.cache.keys().map(String::as_str).collect()
    }

    /// Remove a rule from the store and delete its file.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the rule is not found or cannot be deleted.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let rule = self
            .cache
            .remove(name)
            .ok_or_else(|| StorageError::NotFound(name.into()))?;
        if rule.source_path.exists() {
            std::fs::remove_file(&rule.source_path)?;
        }
        self.index.rebuild(&self.cache);
        Ok(())
    }

    #[must_use]
    pub fn by_trigger(&self, trigger: &str) -> Vec<&str> {
        self.index.by_trigger(trigger)
    }

    #[must_use]
    pub fn by_category(&self, category: &str) -> Vec<&str> {
        self.index.by_category(category)
    }

    /// Check if a rule has changed on disk since it was cached.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the rule is not found or the file cannot be checked.
    pub fn has_changed(&self, name: &str) -> Result<bool> {
        let rule = self
            .cache
            .get(name)
            .ok_or_else(|| StorageError::NotFound(name.into()))?;
        RuleVersion::has_file_changed(rule)
    }
}
