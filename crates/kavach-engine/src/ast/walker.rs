//! Gitignore-aware fast parallel source file walker.

use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Configuration for source file discovery.
#[derive(Debug, Clone)]
pub struct WalkConfig {
    pub hidden: bool,
    pub git_ignore: bool,
    pub custom_excludes: Vec<String>,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            hidden: true,
            git_ignore: true,
            custom_excludes: vec![
                "!target/**".to_string(),
                "!.fastembed_cache/**".to_string(),
                "!node_modules/**".to_string(),
                "!.git/**".to_string(),
            ],
        }
    }
}

/// Check if a file extension is indexable for AST and symbol intelligence.
#[must_use]
pub fn is_indexable(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(
        ext,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "sh" | "bash" | "yaml" | "yml" | "toml" | "json" | "md"
    )
}

/// Discover indexable source files under `root` respecting `.gitignore` rules.
#[must_use]
pub fn walk_source_files(root: &Path, config: &WalkConfig) -> Vec<PathBuf> {
    if root.is_file() {
        return if is_indexable(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        };
    }

    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(config.hidden)
        .parents(true)
        .ignore(true)
        .git_ignore(config.git_ignore)
        .git_global(true)
        .git_exclude(true);

    let mut overrides = OverrideBuilder::new(root);
    for pat in &config.custom_excludes {
        overrides.add(pat).ok();
    }
    if let Ok(ov) = overrides.build() {
        builder.overrides(ov);
    }

    let mut out = Vec::new();
    for entry in builder.build().into_iter().filter_map(Result::ok) {
        let p = entry.path();
        if p.is_file() && is_indexable(p) {
            out.push(p.to_path_buf());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_indexable() {
        assert!(is_indexable(Path::new("src/main.rs")));
        assert!(is_indexable(Path::new("web/app.tsx")));
        assert!(is_indexable(Path::new("script.py")));
        assert!(!is_indexable(Path::new("binary.exe")));
        assert!(!is_indexable(Path::new("photo.png")));
    }
}
