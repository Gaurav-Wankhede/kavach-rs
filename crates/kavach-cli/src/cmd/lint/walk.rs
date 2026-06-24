// Shared recursive .rs walker for lint audit + debt (one walker, two callers).
use std::fs;
use std::path::Path;

const SKIP_DIRS: [&str; 4] = ["target", "node_modules", "dist", ".git"];

/// Visit every `.rs` file under `dir`, calling `f(rel_path, content)` for each.
/// Skips dotfiles + build dirs. Read failures are skipped (best-effort scan).
pub(crate) fn walk_rs(root: &Path, dir: &Path, f: &mut dyn FnMut(&str, &str)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            continue;
        }
        if ft.is_dir() {
            walk_rs(root, &path, f);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
                f(&rel, &content);
            }
        }
    }
}
