//! Dynamic dependency-version reader for package.json + Cargo.toml. No hardcoded
//! list — every dependency is read at gate time. Cargo.toml parsing lives in the
//! `cargo` submodule.
mod cargo;

use cargo::parse_cargo_deps;

/// Extract major version from a semver-like string.
/// Handles: "6.1.0", "^6.1.0", "~6.0", ">=6", "6".
pub(in crate::gates::pre_tool_search) fn extract_major_version(version: &str) -> Option<u32> {
    let trimmed = version
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches(">=")
        .trim_start_matches('>')
        .trim_start_matches('=')
        .trim();
    let first_part = trimmed.split('.').next()?;
    first_part.parse::<u32>().ok()
}

/// Read ALL dependency names + major versions from package.json and Cargo.toml.
pub(super) fn read_all_dep_versions(work_dir: &str) -> Vec<(String, u32)> {
    let mut result: Vec<(String, u32)> = Vec::new();
    let base = std::path::Path::new(work_dir);
    let Ok(body) = std::fs::read_to_string(base.join("package.json")) else {
        let Ok(cargo_body) = std::fs::read_to_string(base.join("Cargo.toml")) else {
            return result;
        };
        parse_cargo_deps(&cargo_body, &mut result);
        return result;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) else {
        return result;
    };
    for section in &["dependencies", "devDependencies"] {
        if let Some(deps) = json.get(*section).and_then(|v| v.as_object()) {
            for (name, ver) in deps {
                if let Some(v_str) = ver.as_str()
                    && let Some(major) = extract_major_version(v_str)
                    && !result.iter().any(|(n, _)| n == name)
                {
                    result.push((name.clone(), major));
                }
            }
        }
    }
    if let Ok(cargo_toml) = std::fs::read_to_string(base.join("Cargo.toml")) {
        parse_cargo_deps(&cargo_toml, &mut result);
    }
    result
}
