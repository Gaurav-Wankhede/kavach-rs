//! YAML-frontmatter relationship parsing: extract `depends_on`/`blocks`/
//! `supersedes`/`references` keys from a fenced (`---`) or loose leading KV
//! block. Hand-rolled to avoid a new dependency.
//! SOURCE: <https://crates.io/crates/markdown-frontmatter> (same `---` convention).

use super::ExtractedRelationship;

const REL_KEYS: &[&str] = &["depends_on", "blocks", "supersedes", "references"];

pub(super) fn extract_frontmatter_rels(content: &str, out: &mut Vec<ExtractedRelationship>) {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return;
    }
    let (start, end) = match (lines.first(), find_closing_fence(&lines)) {
        (Some(&first), Some(close)) if first.trim() == "---" => (1usize, close),
        _ => (0usize, prefix_kv_block(&lines)),
    };
    for line in lines.iter().take(end).skip(start) {
        if let Some((key, val)) = parse_kv(line)
            && REL_KEYS.contains(&key.as_str())
        {
            for tgt in parse_yaml_scalar_or_array(&val) {
                out.push(ExtractedRelationship::new(key.clone(), tgt));
            }
        }
    }
}

fn find_closing_fence(lines: &[&str]) -> Option<usize> {
    if lines.first().map(|l| l.trim()) != Some("---") {
        return None;
    }
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            return Some(i);
        }
    }
    None
}

fn prefix_kv_block(lines: &[&str]) -> usize {
    let mut last: usize = 0;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || parse_kv(line).is_some() {
            last = last.saturating_add(1);
        } else {
            break;
        }
    }
    last
}

fn parse_kv(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let colon = trimmed.find(':')?;
    let key = trimmed.get(..colon)?.trim().to_owned();
    let val = trimmed.get(colon.saturating_add(1)..)?.trim().to_owned();
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some((key, val))
}

fn parse_yaml_scalar_or_array(raw: &str) -> Vec<String> {
    let s = raw.trim();
    if s.is_empty() {
        return Vec::new();
    }
    if let Some(inner) = s.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        return inner
            .split(',')
            .map(|p| p.trim().trim_matches('"').trim_matches('\'').to_owned())
            .filter(|p| !p.is_empty())
            .collect();
    }
    let cleaned = s.trim_matches('"').trim_matches('\'').to_owned();
    if cleaned.is_empty() {
        Vec::new()
    } else {
        vec![cleaned]
    }
}
