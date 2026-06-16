//! Pure parser for a `self-heal` issue body — extracts the incident id +
//! summary from the `[INCIDENT]` block the CI workflow wrote. No IO, fully
//! unit-testable. A body missing the required fields yields `None` (the shell
//! then skips that issue rather than capturing a malformed card).
//! SOURCE: .github/workflows/self-heal.yml issue body shape.

/// The fields the ingestion bridge needs to re-capture an incident locally.
pub(super) struct ParsedIncident {
    /// Incident id (e.g. `ci-run-12345`) → the card key suffix (idempotent).
    pub id: String,
    /// One-line summary → the card title.
    pub summary: String,
}

/// Parse an issue body. Returns `None` unless BOTH `id:` and `summary:` lines
/// are present under the `[INCIDENT]` marker with non-empty values.
pub(super) fn parse_incident(body: &str) -> Option<ParsedIncident> {
    let id = field(body, "id:")?;
    let summary = field(body, "summary:")?;
    if id.is_empty() || summary.is_empty() {
        return None;
    }
    Some(ParsedIncident { id, summary })
}

/// First line beginning with `key` (after trim), returning the trimmed value.
fn field(body: &str, key: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix(key))
        .map(|v| v.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "<!-- self-heal:run:42 -->\n\
        [INCIDENT]\n\
        id: ci-run-42\n\
        summary: build failed on main @ abc12345\n\
        run_url: https://example/run/42\n";

    #[test]
    fn extracts_id_and_summary() {
        let p = parse_incident(BODY).unwrap();
        assert_eq!(p.id, "ci-run-42");
        assert_eq!(p.summary, "build failed on main @ abc12345");
    }

    #[test]
    fn missing_id_is_none() {
        assert!(parse_incident("[INCIDENT]\nsummary: x\n").is_none());
    }

    #[test]
    fn missing_summary_is_none() {
        assert!(parse_incident("[INCIDENT]\nid: ci-run-7\n").is_none());
    }

    #[test]
    fn empty_value_is_none() {
        assert!(parse_incident("id:\nsummary: x\n").is_none());
    }

    #[test]
    fn garbage_body_is_none() {
        assert!(parse_incident("totally unrelated text").is_none());
    }
}
