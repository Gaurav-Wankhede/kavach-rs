//! JS/TS orphan detection: exported items that need importing elsewhere.
use std::path::Path;

/// Flag exported JS/TS items (skips `index`/`mod` barrel files).
pub(super) fn check_js_orphan(file_path: &str, content: &str) -> Option<String> {
    let path = Path::new(file_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem == "index" || stem == "mod" {
        return None;
    }
    let export_count = content
        .lines()
        .filter(|line| {
            let t = line.trim();
            t.starts_with("export function ")
                || t.starts_with("export const ")
                || t.starts_with("export default ")
                || t.starts_with("export class ")
                || t.starts_with("export type ")
                || t.starts_with("export interface ")
        })
        .count();
    if export_count == 0 {
        return None;
    }
    Some(format!(
        "[ORPHAN_GUARD]\nfile: {file_path}\n\n\
         WIRE_CHECK: {export_count} export(s) in `{stem}`\n\
         ACTION: Ensure each is imported at its use site.\n\
         Rule: Create + Import + Use in the SAME turn."
    ))
}
