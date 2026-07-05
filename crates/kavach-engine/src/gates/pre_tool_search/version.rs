//! Stale-version detection: block searches referencing an older major version
//! than what's actually installed (read dynamically from manifests).
use super::deps::read_all_dep_versions;

/// Block searches referencing older major versions than what's actually installed.
/// Reads ALL deps from package.json and Cargo.toml — no hardcoded list.
/// E.g., if package.json has `"astro": "^6.1.0"`, blocks `"Astro 5"` in query.
pub(in crate::gates::pre_tool_search) fn check_stale_version_in_query(
    query: &str,
    work_dir: &str,
) -> Option<String> {
    if work_dir.is_empty() {
        return None;
    }
    let versions = read_all_dep_versions(work_dir);
    if versions.is_empty() {
        return None;
    }
    let query_lower = query.to_lowercase();
    let words: Vec<&str> = query_lower.split_whitespace().collect();
    for pair in words.windows(2) {
        if let [name_part, version_part] = pair {
            let Ok(query_major) = version_part.parse::<u32>() else {
                continue;
            };
            if query_major == 0 {
                continue;
            }
            let clean = name_part.trim_start_matches('@');
            for (dep_name, installed_major) in &versions {
                let dep_lower = dep_name.to_lowercase();
                // Match last path segment for scoped packages (@astrojs/node → node)
                let dep_last = dep_lower.rsplit('/').next().unwrap_or(&dep_lower);
                if (clean == dep_lower || clean == dep_last || *name_part == dep_lower)
                    && query_major < *installed_major
                {
                    return Some(format!(
                        "[VERSION_CURRENCY] Query references \"{name_part} {query_major}\" \
                             but {dep_name} {installed_major} is installed \
                             -> read package.json/Cargo.toml for actual versions and replace \
                             \"{name_part} {query_major}\" with \"{name_part} {installed_major}\" \
                             -> retry."
                    ));
                }
            }
        }
    }
    None
}
