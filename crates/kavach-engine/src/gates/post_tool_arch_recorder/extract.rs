//! Architecture comment extraction.

/// Parsed fields from the structured `// ARCH:` comment block.
pub(super) struct ArchComment {
    pub pattern: String,
    pub scope: String,
    pub cap_choice: Option<String>,
    pub failure_mode: String,
    pub tradeoff: String,
    pub search_year: i64,
    pub search_month: i64,
}

/// Extract the `// ARCH:` structured comment block from file content.
pub(super) fn extract_arch_comment(content: &str) -> Option<ArchComment> {
    let pattern = extract_field(content, "ARCH:")?;
    let scope = extract_field(content, "SCOPE:")?;
    let failure_mode = extract_field(content, "FAILURE_MODE:")?;
    let tradeoff = extract_field(content, "TRADEOFF:")?;

    let cap_choice = extract_field(content, "CAP:");
    let searched_raw = extract_field(content, "SEARCHED:");
    let (search_year, search_month) = parse_searched(searched_raw.as_deref());

    Some(ArchComment {
        pattern,
        scope,
        cap_choice,
        failure_mode,
        tradeoff,
        search_year,
        search_month,
    })
}

pub(super) fn extract_field(content: &str, field_key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("//") {
            continue;
        }
        let after_slashes = trimmed.trim_start_matches('/').trim();
        if let Some(rest) = after_slashes.strip_prefix(field_key) {
            let value = rest.trim().to_owned();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn parse_searched(raw: Option<&str>) -> (i64, i64) {
    raw.map_or_else(
        || (super::time::current_year(), super::time::current_month()),
        |v| {
            let mut parts = v.splitn(2, '-');
            let y = parts
                .next()
                .and_then(|s| s.trim().parse::<i64>().ok())
                .unwrap_or_else(super::time::current_year);
            let m = parts
                .next()
                .and_then(|s| s.trim().parse::<i64>().ok())
                .unwrap_or_else(super::time::current_month);
            (y, m)
        },
    )
}
