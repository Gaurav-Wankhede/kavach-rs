#[derive(Debug, Default)]
pub(crate) struct SkillMetadata {
    pub(crate) description: String,
    pub(crate) triggers: Vec<String>,
    pub(crate) file_patterns: Vec<String>,
}

pub(crate) fn parse_frontmatter(body: &str) -> Option<SkillMetadata> {
    let rest = body.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let yaml = rest.get(..end)?;
    let mut meta = SkillMetadata::default();
    let mut in_file_patterns = false;
    let mut in_metadata = false;
    for line in yaml.lines() {
        let trimmed = line.trim_end();
        if let Some(desc) = trimmed.strip_prefix("description:") {
            meta.description = strip_quotes(desc.trim());
            in_file_patterns = false;
            in_metadata = false;
            continue;
        }
        if trimmed == "file_patterns:" {
            in_file_patterns = true;
            in_metadata = false;
            continue;
        }
        if trimmed == "metadata:" {
            in_metadata = true;
            in_file_patterns = false;
            continue;
        }
        if in_file_patterns {
            if let Some(item) = trimmed.strip_prefix("  - ") {
                meta.file_patterns.push(strip_quotes(item.trim()));
                continue;
            }
            if !trimmed.starts_with(' ') {
                in_file_patterns = false;
            }
        }
        if in_metadata {
            if let Some(triggers) = trimmed.strip_prefix("  triggers:") {
                meta.triggers = parse_inline_list(triggers.trim());
                continue;
            }
            if !trimmed.starts_with(' ') {
                in_metadata = false;
            }
        }
    }
    if meta.triggers.is_empty() {
        meta.triggers = extract_trigger_phrase(&meta.description);
    }
    Some(meta)
}

fn strip_quotes(s: &str) -> String {
    let trimmed = s.trim();
    let without_leading = trimmed.strip_prefix('"').map_or(trimmed, |t| t);
    let without_trailing = without_leading
        .strip_suffix('"')
        .map_or(without_leading, |t| t);
    without_trailing.to_owned()
}

fn parse_inline_list(s: &str) -> Vec<String> {
    let trimmed = s.trim().trim_start_matches('[').trim_end_matches(']');
    trimmed
        .split(',')
        .map(|t| t.trim().trim_matches('"').to_owned())
        .filter(|t| !t.is_empty())
        .collect()
}

fn extract_trigger_phrase(description: &str) -> Vec<String> {
    const MARKER: &str = "Trigger on:";
    let idx = match description.find(MARKER) {
        Some(i) => i.saturating_add(MARKER.len()),
        None => return Vec::new(),
    };
    let Some(tail) = description.get(idx..) else {
        return Vec::new();
    };
    let end = tail.find('.').map_or(tail.len(), |i| i);
    let Some(phrase) = tail.get(..end) else {
        return Vec::new();
    };
    phrase
        .split(',')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}
