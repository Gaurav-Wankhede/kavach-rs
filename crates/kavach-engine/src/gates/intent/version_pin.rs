mod registry;
mod npm;
mod pypi;

use registry::{crates_index_url, npm_url, pypi_url};
use npm::npm_deps;
use pypi::pypi_deps;

fn find_lockfile() -> Option<String> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("Cargo.lock");
        if candidate.is_file() {
            return std::fs::read_to_string(candidate).ok();
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn parse_versions(lock: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut name: Option<&str> = None;
    for line in lock.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("name = \"") {
            name = rest.strip_suffix('"');
        } else if let Some(rest) = t.strip_prefix("version = \"")
            && let Some(v) = rest.strip_suffix('"')
            && let Some(n) = name.take()
        {
            out.push((n.to_owned(), v.to_owned()));
        }
    }
    out
}

fn prompt_mentions(prompt_lc: &str, crate_name: &str) -> bool {
    let needle = crate_name.to_lowercase();
    let alt = needle.replace('-', "_");
    prompt_lc.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').any(|tok| {
        let tok = tok.trim_matches(|c| c == '-' || c == '_');
        tok == needle || tok.replace('-', "_") == alt
    })
}

pub(super) fn version_pin_block(prompt: &str) -> String {
    let Some(lock) = find_lockfile() else {
        return String::new();
    };
    let prompt_lc = prompt.to_lowercase();
    let mut hits: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (name, version) in parse_versions(&lock) {
        if prompt_mentions(&prompt_lc, &name) && seen.insert(name.clone()) {
            hits.push(format!("  {name} = {version} (installed; pin research to THIS)"));
        }
    }
    if let Ok(work_dir) = std::env::current_dir() {
        for (name, version) in npm_deps(&work_dir) {
            if prompt_mentions(&prompt_lc, &name) && seen.insert(name.clone()) {
                hits.push(format!("  {name} = {version} (npm; pin research to THIS)"));
            }
        }
        for (name, version) in pypi_deps(&work_dir) {
            if prompt_mentions(&prompt_lc, &name) && seen.insert(name.clone()) {
                hits.push(format!("  {name} = {version} (python; pin research to THIS)"));
            }
        }
    }
    if hits.is_empty() {
        return String::new();
    }
    let confirm: Vec<String> = seen
        .iter()
        .map(|name| {
            let crates = crates_index_url(name);
            let npm = npm_url(name);
            let pypi = pypi_url(name);
            format!("  {name}: crates={crates} npm={npm} pypi={pypi} (fetch, do not assume)")
        })
        .collect();
    let installed = hits.join("\n");
    let upstream = confirm.join("\n");
    format!(
        "\n[VERSION_PIN] installed versions (FACTS). The LATEST upstream \
         version is NOT a fact you may recall from training weights — you MUST fetch it \
         from the registry BEFORE claiming any version is newest/latest:\nINSTALLED:\n{installed}\n\
         CONFIRM-UPSTREAM (fetch, do not assume):\n{upstream}\n"
    )
}

#[cfg(test)]
#[path = "version_pin/version_pin_test.rs"]
mod version_pin_test;
