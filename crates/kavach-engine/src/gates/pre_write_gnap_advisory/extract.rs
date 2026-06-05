//! Spec-section extraction keyed by the auth concepts present in the content.

/// Pull the GNAP spec sections relevant to the concepts mentioned in `content`.
/// Returns concatenated markdown (capped at 4000 chars), empty when none match.
pub(super) fn extract_relevant_sections(spec: &str, content: &str) -> String {
    // (concept-predicate any-of, section start marker, section end marker).
    const RULES: &[(&[&str], &str, &str)] = &[
        (
            &["grant", "access_token"],
            "## 1. Grant Request/Response",
            "## 2.",
        ),
        (
            &["interact", "redirect"],
            "## 2. Interaction Modes",
            "## 3.",
        ),
        (
            &["httpsig", "signature", "proof"],
            "## 3. Key Proofing",
            "## 4.",
        ),
        (
            &["introspect", "resource_server"],
            "## 4. Resource Server",
            "## 5.",
        ),
        (&["error", "invalid_"], "## 6. Error Handling", "## 7."),
        (
            &["struct", "impl"],
            "### 8.1 Grant Request Types",
            "### 8.2",
        ),
    ];
    let content_lower = content.to_lowercase();
    let mut sections = String::new();
    for (needles, start, end) in RULES {
        if needles.iter().any(|n| content_lower.contains(n))
            && let Some(section) = extract_section(spec, start, end)
        {
            sections.push_str(&section);
            sections.push('\n');
        }
    }
    let max_len = 4000;
    if sections.len() > max_len {
        sections.truncate(max_len);
        sections.push_str("\n[TRUNCATED — read full spec for complete types]\n");
    }
    sections
}

/// Slice the spec from `start_marker` to `end_marker` (cap 2000 chars).
fn extract_section(spec: &str, start_marker: &str, end_marker: &str) -> Option<String> {
    let start = spec.find(start_marker)?;
    let remainder = spec.get(start..)?;
    let end = remainder.find(end_marker).unwrap_or(remainder.len());
    let truncated_len = end.min(2000);
    Some(remainder.get(..truncated_len)?.to_owned())
}
