/// Split a PEP 508 requirement into (name, version-spec); no operator → empty version.
fn split_requirement(spec: &str) -> Option<(String, String)> {
    let s = spec.trim();
    if s.is_empty() {
        return None;
    }
    let cut = s.find(['=', '>', '<', '!', '~']);
    let Some(pos) = cut else {
        return Some((s.to_owned(), String::new()));
    };
    let (name, rest) = s.split_at(pos);
    let name = name.trim_end();
    if name.is_empty() {
        return None;
    }
    let version = rest.strip_prefix("==").unwrap_or(rest).trim();
    Some((name.to_owned(), version.to_owned()))
}

pub(super) fn parse_requirements_txt(body: &str) -> Vec<(String, String)> {
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('-'))
        .filter_map(split_requirement)
        .collect()
}

pub(super) fn parse_pyproject_deps(body: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("dependencies") {
            continue;
        }
        let Some((_, after)) = trimmed.split_once('[') else {
            continue;
        };
        let Some((inside, _)) = after.split_once(']') else {
            continue;
        };
        for item in inside.split(',') {
            let item = item.trim().trim_matches(|c| c == '"' || c == '\'');
            if let Some(pair) = split_requirement(item) {
                result.push(pair);
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
