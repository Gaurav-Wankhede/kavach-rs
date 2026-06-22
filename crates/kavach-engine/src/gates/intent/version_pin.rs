//! `[VERSION_PIN]` directive: hand the LLM the EXACT installed version of any
//! dependency named in the prompt, read from Cargo.lock — so a research query is
//! pinned to ground truth, never to stale training weights.
//! SOURCE: decision.research.version-pin-from-lockfile

/// Walk up from cwd to the nearest `Cargo.lock`; return its text.
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

/// Parse `name = "x"` / `version = "y"` pairs from a Cargo.lock into (name, version).
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

/// True when `crate_name` appears as a whole token in the lowercased prompt
/// (hyphen/underscore-insensitive), so `surrealdb` matches but a substring does not.
fn prompt_mentions(prompt_lc: &str, crate_name: &str) -> bool {
    let needle = crate_name.to_lowercase();
    let alt = needle.replace('-', "_");
    prompt_lc.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').any(|tok| {
        let tok = tok.trim_matches(|c| c == '-' || c == '_');
        tok == needle || tok.replace('-', "_") == alt
    })
}

/// Build a `[VERSION_PIN]` block for every lockfile crate named in the prompt.
/// Empty string when no dependency is mentioned or no lockfile is found.
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
    if hits.is_empty() {
        return String::new();
    }
    format!(
        "\n[VERSION_PIN] installed dependency versions from Cargo.lock — research these \
         EXACT versions, never a version from memory:\n{}\n",
        hits.join("\n")
    )
}

#[cfg(test)]
#[path = "version_pin_test.rs"]
mod version_pin_test;
