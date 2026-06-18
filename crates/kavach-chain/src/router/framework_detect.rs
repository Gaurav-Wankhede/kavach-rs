use std::collections::HashSet;

#[must_use]
pub fn extract_framework_from_task(task: &str) -> Vec<String> {
    let patterns = load_framework_patterns();
    if patterns.is_empty() {
        return Vec::new();
    }
    let escaped: Vec<String> = patterns.iter().map(|p| regex::escape(p)).collect();
    let re_str = format!("(?i)({})", escaped.join("|"));
    let re = match regex::Regex::new(&re_str) {
        Ok(r) => r,
        Err(e) => {
            // Pattern source is user-editable framework config — a bad pattern
            // silently disables the framework router. Surface so operators see
            // the degraded routing instead of inferring "no frameworks matched".
            use std::io::Write;
            drop(writeln!(
                std::io::stderr(),
                "[kavach-chain] framework_detect: regex compile failed ({e}); routing disabled"
            ));
            return Vec::new();
        }
    };
    let mut seen = HashSet::new();
    for m in re.find_iter(task) {
        let lower = m.as_str().to_lowercase();
        seen.insert(lower);
    }
    seen.into_iter().collect()
}

pub(crate) fn load_framework_patterns() -> Vec<String> {
    let sections = kavach_config::get_framework_patterns();
    let all: Vec<String> = sections.values().flatten().cloned().collect();
    if all.is_empty() {
        return default_framework_patterns();
    }
    all
}

fn default_framework_patterns() -> Vec<String> {
    [
        "axum",
        "tonic",
        "tokio",
        "react",
        "vue",
        "angular",
        "dioxus",
        "leptos",
        "yew",
        "astro",
        "tauri",
        "postgres",
        "sqlx",
        "diesel",
        "prisma",
        "terraform",
        "kubernetes",
        "docker",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_framework() {
        let fws = extract_framework_from_task("build an axum REST API with tokio");
        assert!(fws.contains(&"axum".to_owned()));
        assert!(fws.contains(&"tokio".to_owned()));
    }
}
