use std::path::{Path, PathBuf};

pub(super) fn source_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return if is_scannable_rust(&root.to_string_lossy()) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    let mut overrides = ignore::overrides::OverrideBuilder::new(root);
    for pat in &["!target/**", "!node_modules/**", "!dist/**", "!build/**", "!.venv/**", "!.git/**"] {
        overrides.add(pat).ok();
    }
    if let Ok(ov) = overrides.build() {
        builder.overrides(ov);
    }

    let mut out: Vec<PathBuf> = builder
        .build()
        .flatten()
        .filter_map(|entry| {
            let p = entry.path();
            if p.is_file() && is_scannable_rust(&p.to_string_lossy()) {
                Some(p.to_path_buf())
            } else {
                None
            }
        })
        .collect();
    out.sort();
    out
}

pub(super) fn is_scannable_rust(path: &str) -> bool {
    let p = Path::new(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("rs")) {
        return false;
    }
    let s = path.replace('\\', "/");
    if s.contains("/tests/") || s.starts_with("tests/") {
        return false;
    }
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name != "tests.rs" && !name.trim_end_matches(".rs").contains("_test"))
}
