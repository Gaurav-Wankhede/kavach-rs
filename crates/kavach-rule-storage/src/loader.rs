//! Load TOON rule files from a directory on disk.

use std::fs;
use std::path::Path;

use crate::error::{Result, StorageError};
use crate::store::StoredRule;
use crate::version::RuleVersion;

/// Scan a directory for .toon and SKILL.md files, parse each into `StoredRule`.
///
/// # Errors
/// Returns [`StorageError`] if the directory read fails or a rule file cannot be parsed.
pub fn load_rules_from_dir(dir: &Path) -> Result<Vec<StoredRule>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut rules = Vec::new();
    let entries = fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if is_rule_file(&path) {
            try_load_rule(&path, &mut rules);
            continue;
        }
        // Descend one level into subdirectories for SKILL.md
        if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                try_load_rule(&skill_md, &mut rules);
            }
        }
    }
    Ok(rules)
}

fn try_load_rule(path: &Path, rules: &mut Vec<StoredRule>) {
    if let Ok(rule) = load_single_rule(path) {
        rules.push(rule);
    }
    // Malformed files are silently skipped; caller may diagnose via explicit verify step
}

fn is_rule_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    ext == Some("toon") || name == "SKILL.md"
}

fn load_single_rule(path: &Path) -> Result<StoredRule> {
    let content = fs::read_to_string(path)?;
    let doc =
        kavach_toon::parse_string(&content).map_err(|e| StorageError::ParseError(e.to_string()))?;
    let sections = kavach_rule_parser::extract_sections(&doc);
    let fm = kavach_rule_parser::parse_frontmatter(&content)
        .map_err(|e| StorageError::FrontmatterError(e.to_string()))?;
    let research = match &sections.research_gate {
        Some(_) => kavach_rule_ast::ResearchGate {
            mandatory: true,
            rule: "WebSearch before implementing".into(),
        },
        None => kavach_rule_ast::ResearchGate {
            mandatory: false,
            rule: String::new(),
        },
    };
    let file_patterns = fm.effective_patterns().to_vec();
    let definition = kavach_rule_ast::SkillDefinition {
        metadata: kavach_rule_ast::SkillMetadata {
            name: fm.name.clone(),
            description: fm.description,
            protocol: fm.compatibility,
            triggers: fm.metadata.triggers,
            file_patterns,
            priority: kavach_rule_ast::SkillPriority::parse_str(&fm.priority),
        },
        research_gate: research,
    };
    let hash = RuleVersion::compute_hash(&content);
    let modified = file_modified_iso(path)?;
    Ok(StoredRule {
        definition,
        source_path: path.to_path_buf(),
        content_hash: hash,
        last_modified: modified,
        version: 1,
    })
}

fn file_modified_iso(path: &Path) -> Result<String> {
    let meta = fs::metadata(path)?;
    let modified = meta.modified()?;
    let dt: chrono::DateTime<chrono::Local> = modified.into();
    Ok(dt.format("%Y-%m-%dT%H:%M:%S").to_string())
}

// writeln_stderr helper removed — try_load_rule now uses eprintln! directly
// (RFC 1869 canonical CLI stderr macro, no Result discard needed).
