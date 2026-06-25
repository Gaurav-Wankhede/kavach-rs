pub(super) fn parse_requirements_txt(body: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }
        let operators = ['=', '>', '<', '!', '~'];
        if let Some(pos) = trimmed.find(|c| operators.contains(&c)) {
            let name = trimmed[..pos].trim_end();
            let version = &trimmed[pos..];
            if !name.is_empty() {
                result.push((name.to_owned(), version.to_owned()));
            }
        }
    }
    result
}

pub(super) fn parse_pyproject_deps(body: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut in_deps = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("dependencies") && trimmed.contains('[') {
            in_deps = true;
        }
        if in_deps {
            if trimmed.starts_with(']') {
                break;
            }
            if let Some(quoted) = trimmed.strip_prefix('"').and_then(|s| s.split_once('"')) {
                let dep_spec = quoted.0;
                let operators = ['=', '>', '<', '!', '~'];
                if let Some(pos) = dep_spec.find(|c| operators.contains(&c)) {
                    let name = dep_spec[..pos].trim_end();
                    let version = &dep_spec[pos..];
                    if !name.is_empty() {
                        result.push((name.to_owned(), version.to_owned()));
                    }
                } else if !dep_spec.is_empty() {
                    result.push((dep_spec.to_owned(), String::new()));
                }
            }
        }
    }
    result
}

pub(super) fn pypi_deps(work_dir: &std::path::Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Ok(body) = std::fs::read_to_string(work_dir.join("requirements.txt")) {
        result.extend(parse_requirements_txt(&body));
    }
    if let Ok(body) = std::fs::read_to_string(work_dir.join("pyproject.toml")) {
        result.extend(parse_pyproject_deps(&body));
    }
    result
}

#[cfg(test)]
#[path = "pypi_test.rs"]
mod pypi_test;
