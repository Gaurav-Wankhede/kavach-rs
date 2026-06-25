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
            let mut version = trimmed[pos..].to_owned();
            if version.starts_with("==") {
                version = version[2..].to_owned();
            }
            if !name.is_empty() {
                result.push((name.to_owned(), version));
            }
        }
    }
    result
}

pub(super) fn parse_pyproject_deps(body: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("dependencies") && trimmed.contains('[') && trimmed.contains(']') {
            if let Some(start) = trimmed.find('[') {
                if let Some(end) = trimmed.find(']') {
                    let array_content = &trimmed[start + 1..end];
                    for item in array_content.split(',') {
                        let item = item.trim().trim_matches(|c| c == '"' || c == '\'' || c == ' ');
                        if !item.is_empty() {
                            let operators = ['=', '>', '<', '!', '~'];
                            if let Some(pos) = item.find(|c| operators.contains(&c)) {
                                let name = item[..pos].trim_end();
                                let mut version = item[pos..].to_owned();
                                if version.starts_with("==") {
                                    version = version[2..].to_owned();
                                }
                                if !name.is_empty() {
                                    result.push((name.to_owned(), version));
                                }
                            } else if !item.is_empty() {
                                result.push((item.to_owned(), String::new()));
                            }
                        }
                    }
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
