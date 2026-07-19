//! Serialize rules to TOON format and write atomically to disk.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::error::{Result, StorageError};
use crate::store::StoredRule;

/// Write a `StoredRule` to disk as a TOON file. Uses atomic write (temp + rename).
///
/// # Errors
/// Returns [`StorageError`] if the rule cannot be serialized or written to disk.
pub fn write_rule(dir: &Path, rule: &StoredRule) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let file_name = format!("{}.toon", rule.definition.metadata.name);
    let final_path = dir.join(&file_name);
    let tmp_path = dir.join(format!(".{file_name}.tmp"));
    let content = serialize_to_skill(rule);
    write_atomic(&tmp_path, &final_path, content.as_bytes())?;
    Ok(final_path)
}

fn write_atomic(tmp: &Path, final_path: &Path, data: &[u8]) -> Result<()> {
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(tmp)?;
    file.try_lock_exclusive()
        .map_err(|e| StorageError::LockFailed(e.to_string()))?;
    let mut writer = std::io::BufWriter::new(&file);
    writer.write_all(data)?;
    writer.flush()?;
    file.unlock()
        .map_err(|e| StorageError::LockFailed(e.to_string()))?;
    drop(writer);
    drop(file);
    fs::rename(tmp, final_path).map_err(|source| StorageError::AtomicRename { source })?;
    Ok(())
}

fn serialize_to_skill(rule: &StoredRule) -> String {
    let meta = &rule.definition.metadata;
    let rg = &rule.definition.research_gate;
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut out = String::with_capacity(512);
    out.push_str("---\n");
    out.push_str("name: ");
    out.push_str(&meta.name);
    out.push_str("\ndescription: ");
    out.push_str(&meta.description);
    out.push_str("\nlicense: MIT\ncompatibility: ");
    out.push_str(&meta.protocol);
    out.push_str("\n---\n\n[META]\nprotocol: ");
    out.push_str(&meta.protocol);
    out.push_str("\ndate: ");
    out.push_str(&date);
    out.push_str("\nversion: ");
    out.push_str(&rule.version.to_string());
    out.push_str("\n\n");
    if !meta.triggers.is_empty() {
        out.push_str("[TRIGGERS]\n");
        for t in &meta.triggers {
            out.push_str("trigger: ");
            out.push_str(t);
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str("[RESEARCH_GATE]\nmandatory: ");
    out.push_str(if rg.mandatory { "true" } else { "false" });
    out.push_str("\nrule: ");
    out.push_str(&rg.rule);
    out.push_str("\n\n");
    out
}
