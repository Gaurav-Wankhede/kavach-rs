use std::fs;
use std::path::Path;

pub(crate) fn walk_rs(root: &Path, _dir: &Path, f: &mut dyn FnMut(&str, &str)) {
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    let mut overrides = ignore::overrides::OverrideBuilder::new(root);
    for pat in &["!target/**", "!node_modules/**", "!dist/**", "!.git/**"] {
        overrides.add(pat).ok();
    }
    if let Ok(ov) = overrides.build() {
        builder.overrides(ov);
    }

    for entry in builder.build().flatten() {
        let path = entry.path();
        if path.is_file()
            && path.extension().is_some_and(|e| e == "rs")
            && let Ok(content) = fs::read_to_string(path)
        {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            f(&rel, &content);
        }
    }
}
