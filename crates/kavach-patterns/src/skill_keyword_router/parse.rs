use super::types::{ModelTier, SkillContext, SkillMetadata};

pub(super) fn extract_frontmatter(content: &str) -> Option<Vec<&str>> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first() != Some(&"---") {
        return None;
    }
    let end_idx = lines.iter().skip(1).position(|l| *l == "---")?;
    Some(lines.get(1..=end_idx)?.to_vec())
}

pub(super) fn extract_metadata(content: &str) -> SkillMetadata {
    let Some(frontmatter) = extract_frontmatter(content) else {
        return SkillMetadata::default();
    };

    let mut metadata = SkillMetadata::default();
    for line in frontmatter {
        let trimmed = line.trim();
        if trimmed.starts_with("context:") {
            let val = trimmed.strip_prefix("context:").unwrap_or("").trim();
            metadata.context = match val {
                "fork" => SkillContext::Fork,
                _ => SkillContext::Inline,
            };
        }
        if trimmed.starts_with("agent:") {
            let val = trimmed.strip_prefix("agent:").unwrap_or("").trim();
            if !val.is_empty() {
                metadata.agent = Some(val.to_owned());
            }
        }
        if trimmed.starts_with("model_tier:") {
            let val = trimmed.strip_prefix("model_tier:").unwrap_or("").trim();
            if let Some(tier) = ModelTier::parse(val) {
                metadata.model_tier = tier;
            }
        }
    }
    metadata
}

pub(super) fn extract_keywords(content: &str) -> Vec<String> {
    let Some(frontmatter) = extract_frontmatter(content) else {
        return Vec::new();
    };

    let mut keywords = Vec::new();
    for line in frontmatter {
        if line.trim().starts_with("triggers:") {
            keywords.extend(extract_triggers_array(line));
        }
        if line.contains("description:") {
            keywords.extend(extract_trigger_on(line));
        }
    }
    keywords
}

pub(super) fn extract_triggers_array(line: &str) -> Vec<String> {
    let Some(start) = line.find('[') else {
        return Vec::new();
    };
    let Some(end) = line.find(']') else {
        return Vec::new();
    };
    let Some(arr) = line.get(start.saturating_add(1)..end) else {
        return Vec::new();
    };
    arr.split(',')
        .map(|kw| kw.trim().trim_matches('"').trim_matches('\'').to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn extract_trigger_on(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    let markers = ["trigger on:", "invoke on:", "triggers on:"];
    let (pos, marker_len) = markers
        .iter()
        .find_map(|m| lower.find(m).map(|p| (p, m.len())))
        .unwrap_or((0, 0));
    if marker_len == 0 {
        return Vec::new();
    }
    let Some(after) = line.get(pos.saturating_add(marker_len)..) else {
        return Vec::new();
    };
    let end = after.find('"').unwrap_or(after.len());
    let Some(slice) = after.get(..end) else {
        return Vec::new();
    };
    slice
        .split(',')
        .map(|kw| kw.trim().trim_matches('.').to_owned())
        .filter(|s| s.len() > 2)
        .collect()
}
