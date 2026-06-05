// ARCH: see kavach db get --category decision --key arch.decision.silent_io_guard_shipped
// ALGO: detect_patterns + generate_skill (preserved verbatim from kavach_rule_generator crate; not modified by this silent-IO migration). SOURCE: crates/kavach-rule-generator
use std::path::Path;

use kavach_rule_generator::{detect_patterns, emit_skill, generate_skill};

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

#[expect(
    clippy::float_arithmetic,
    reason = "confidence is 0.0..1.0; multiplication by 100.0 for display percentage is intentional"
)]
pub(super) fn run(dir: &str) -> i32 {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        let msg = format!("rules generate: not a directory: {dir}");
        if let Err(e) = ewrite_or_exit(&msg) {
            return into_exit_code(e);
        }
        return 1;
    }
    let (files, content) = scan_directory(dir_path);
    let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let patterns = detect_patterns(&file_refs, &content);
    if patterns.is_empty() {
        let msg = format!("No patterns detected in: {dir}");
        if let Err(e) = print_or_exit(&msg) {
            return into_exit_code(e);
        }
        return 0;
    }
    let header = format!("Detected {} pattern(s):", patterns.len());
    if let Err(e) = print_or_exit(&header) {
        return into_exit_code(e);
    }
    for pat in &patterns {
        let line = format!(
            "  {:?} — {} (confidence: {:.0}%)",
            pat.pattern_type,
            pat.name,
            pat.confidence * 100.0
        );
        if let Err(e) = print_or_exit(&line) {
            return into_exit_code(e);
        }
        let skill = generate_skill(pat);
        let toon = emit_skill(&skill);
        if let Err(e) = print_or_exit("--- Generated TOON ---") {
            return into_exit_code(e);
        }
        if let Err(e) = print_or_exit(&toon) {
            return into_exit_code(e);
        }
    }
    0
}

fn scan_directory(dir: &Path) -> (Vec<String>, String) {
    let mut files = Vec::new();
    let mut content = String::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (files, content);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            files.push(path.display().to_string());
            if let Ok(text) = std::fs::read_to_string(&path)
                && content.len() < 8192
            {
                let truncated = text.chars().take(2048).collect::<String>();
                content.push_str(&truncated);
            }
        }
    }
    (files, content)
}
