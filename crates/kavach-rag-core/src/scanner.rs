use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A single discovered source document.
///
/// Contains a stable id (used as the tree root id) and full textual body.
/// Ids are derived from the path relative to the scan root so emitted trees
/// remain portable across machines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedDoc {
    id: String,
    path: PathBuf,
    body: String,
}

impl ScannedDoc {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Recursively scan `root` and return every file whose extension is in `allowed_exts`.
///
/// Extensions are lowercased, without leading dot. Symlinks and hidden directories
/// (leading `.`) are skipped so the scan never follows build caches, `.git`, or backup trees.
///
/// # Errors
///
/// Returns an error if the root path cannot be canonicalized or if a directory
/// read operation fails.
pub fn scan_dir(root: &Path, allowed_exts: &[&str]) -> io::Result<Vec<ScannedDoc>> {
    let mut out: Vec<ScannedDoc> = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    let canonical = fs::canonicalize(root)?;
    walk(&canonical, &canonical, allowed_exts, &mut out)?;
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn walk(
    root: &Path,
    dir: &Path,
    allowed_exts: &[&str],
    out: &mut Vec<ScannedDoc>,
) -> io::Result<()> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk(root, &path, allowed_exts, out)?;
            continue;
        }
        if !has_allowed_ext(&path, allowed_exts) {
            continue;
        }
        let Ok(body) = fs::read_to_string(&path) else {
            continue;
        };
        let id = relative_id(root, &path);
        out.push(ScannedDoc { id, path, body });
    }
    Ok(())
}

fn has_allowed_ext(path: &Path, allowed: &[&str]) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };
    allowed.iter().any(|a| *a == ext)
}

fn relative_id(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| path.to_string_lossy().to_string(),
        |rel| rel.to_string_lossy().replace('\\', "/"),
    )
}
