//! `kavach origin <NAME> [path]` — deterministic symbol-origin / config-source finder.

mod matcher;
mod query;
mod refine;
mod role_query;
mod scorer;
mod secret_hints;
mod signals;
mod site;
mod walker;

use site::Site;
use std::path::{Path, PathBuf};

/// `kavach origin --query '<json>'` entry — dynamic multi-signal role resolver.
#[must_use]
pub(crate) fn run_query(json: &str, root: &Path, all: bool) -> i32 {
    query::run(json, root, all)
}

/// `kavach origin` entry. Exit: 0 = found, 1 = no declaration site, 2 = bad root.
#[must_use]
pub(crate) fn run(name: &str, root: &Path, all: bool) -> i32 {
    if name.is_empty() {
        eprintln!("origin: empty symbol name");
        return 2;
    }
    if !root.exists() {
        eprintln!("origin: target path missing: {}", root.display());
        return 2;
    }
    let mut sites = collect_sites(name, root);
    sites.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| (a.file.clone(), a.line).cmp(&(b.file.clone(), b.line)))
    });
    sites.dedup_by(|a, b| a.dedup_key() == b.dedup_key());
    report(name, &sites, root, all);
    i32::from(sites.is_empty())
}

fn collect_sites(name: &str, root: &Path) -> Vec<Site> {
    let mut out = Vec::new();
    for path in source_files(root) {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        // SOURCE: rust-lang.github.io/rust-clippy/master/index.html#map_unwrap_or
        let rel = path.strip_prefix(root).ok().filter(|r| !r.as_os_str().is_empty())
            .map_or_else(|| path.clone(), std::path::PathBuf::from);
        out.extend(matcher::sites_in(name, &rel.to_string_lossy(), &src));
    }
    out
}

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "dist", "build", ".venv"];

fn source_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return if kavach_patterns::is_code_file(&root.to_string_lossy()) { vec![root.to_path_buf()] } else { Vec::new() };
    }
    let mut out = Vec::new();
    collect(root, &mut out);
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(dir_iter) = std::fs::read_dir(dir) else {
        return;
    };
    for item in dir_iter.flatten() {
        let path = item.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if !skip {
                collect(&path, out);
            }
        } else if kavach_patterns::is_code_file(&path.to_string_lossy()) {
            out.push(path);
        }
    }
}

fn report(name: &str, sites: &[Site], root: &Path, all: bool) {
    if sites.is_empty() {
        println!("[KAVACH_ORIGIN] {name}: no declaration site found (it may be imported, generated, or a usage only)");
        if let Some(hint) = refine::tool_hint(root) {
            println!("  {hint}");
        }
        return;
    }
    let Some(top) = sites.first() else { return };
    let tag = if top.kind.is_centralized() {
        " (centralized)"
    } else {
        ""
    };
    println!(
        "[KAVACH_ORIGIN] {name} -> {} at {}:{}{tag}",
        top.kind.label(),
        top.file,
        top.line
    );
    if all {
        for site in sites.iter().skip(1) {
            println!("  also: {} at {}:{}", site.kind.label(), site.file, site.line);
        }
    } else {
        let extra = sites.len().saturating_sub(1);
        if extra > 0 {
            println!("  +{extra} more — pass --all to list");
        }
    }
}

#[cfg(test)]
#[path = "origin_test.rs"]
mod origin_test;
