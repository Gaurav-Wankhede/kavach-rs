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
mod cache;

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

    if !all
        && let Some(cached_site) = cache::get(name, root)
    {
        report(name, &[cached_site], root, false);
        return 0;
    }

    let mut sites = collect_sites(name, root);
    sites.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| (a.file.clone(), a.line).cmp(&(b.file.clone(), b.line)))
    });
    sites.dedup_by(|a, b| a.dedup_key() == b.dedup_key());

    if let Some(top) = sites.first() {
        cache::put(name, top);
    }

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
        let rel = path
            .strip_prefix(root)
            .ok()
            .filter(|r| !r.as_os_str().is_empty())
            .map_or_else(|| path.clone(), PathBuf::from);
        out.extend(matcher::sites_in(name, &rel.to_string_lossy(), &src));
    }
    out
}

const ORIGIN_EXTENSIONS: &[&str] = &[
    // Modern Code
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "hpp", "cs", "swift", "kt", "rb", "php", "zig", "lua",
    // Config & Secrets
    "env", "toml", "yaml", "yml", "json", "jsonc", "ini", "properties", "conf", "cfg",
    // Schemas & Databases
    "sql", "ddl", "surql", "prisma", "proto", "graphql", "gql",
    // Documentation & Specs
    "md", "markdown", "mdx", "adoc", "org",
    // Web & Markup
    "html", "xml", "svg", "vue", "svelte", "astro", "css", "scss",
    // Infra & Toolchains
    "tf", "hcl", "dockerfile", "containerfile", "makefile", "justfile",
];

fn is_origin_indexable(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if name == "dockerfile" || name == "containerfile" || name == "makefile" || name == "justfile" || name.starts_with(".env") {
        return true;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .is_some_and(|e| ORIGIN_EXTENSIONS.contains(&e.as_str()))
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return if is_origin_indexable(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    let mut out = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .parents(true)
        .ignore(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for result in walker {
        let Ok(entry) = result else {
            continue;
        };
        let path = entry.path();
        if path.is_file() && is_origin_indexable(path) {
            out.push(path.to_path_buf());
        }
    }
    out
}

fn report(name: &str, sites: &[Site], root: &Path, all: bool) {
    if sites.is_empty() {
        println!(
            "[KAVACH_ORIGIN] {name}: no declaration site found (it may be imported, generated, or a usage only)"
        );
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
            println!(
                "  also: {} at {}:{}",
                site.kind.label(),
                site.file,
                site.line
            );
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
