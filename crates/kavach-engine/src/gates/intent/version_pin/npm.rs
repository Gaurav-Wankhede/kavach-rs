pub(super) fn parse_package_json(body: &str) -> Vec<(String, String)> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for section in &["dependencies", "devDependencies"] {
        if let Some(deps) = json.get(*section).and_then(|v| v.as_object()) {
            for (name, ver) in deps {
                if let Some(v_str) = ver.as_str() {
                    result.push((name.clone(), v_str.to_owned()));
                }
            }
        }
    }
    result
}

pub(super) fn npm_deps(work_dir: &std::path::Path) -> Vec<(String, String)> {
    let Ok(body) = std::fs::read_to_string(work_dir.join("package.json")) else {
        return Vec::new();
    };
    parse_package_json(&body)
}

#[cfg(test)]
#[path = "npm_test.rs"]
mod npm_test;
