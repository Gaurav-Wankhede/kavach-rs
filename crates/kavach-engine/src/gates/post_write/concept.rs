//! `// CONCEPT:` marker scanner — upserts L0 concepts to the graph via RPC.
//!
//! ALGO `LineByLineFieldExtract`; `PROBLEM_CLASS` `structured_comment_parse`.
//! Rejected regex (compile per Write) and nom (overkill). TIME O(n), SPACE O(k).
//! BENCHMARK <https://doc.rust-lang.org/std/str/struct.Lines.html>.

const CONCEPT_MARKER_CAP: usize = 8;

/// Scan up to `CONCEPT_MARKER_CAP` `// CONCEPT:` lines and upsert each concept.
/// Returns the number of valid concepts upserted.
pub(super) fn scan_concept_markers(content: &str) -> usize {
    let mut count = 0usize;
    for line in content.lines() {
        if count >= CONCEPT_MARKER_CAP {
            return count;
        }
        let Some(body) = line.trim_start().strip_prefix("// CONCEPT:") else {
            continue;
        };
        if upsert_concept(body.trim()) {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Parse one marker body (`name | DESC: .. | TAGS: a,b`) and upsert it.
/// Returns true when a valid concept was sent.
fn upsert_concept(body: &str) -> bool {
    let (mut name, mut desc, mut tags) = (String::new(), String::new(), Vec::<String>::new());
    for part in body.split('|') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("DESC:") {
            rest.trim().clone_into(&mut desc);
        } else if let Some(rest) = part.strip_prefix("TAGS:") {
            tags = rest
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect();
        } else if name.is_empty() {
            name.push_str(part);
        }
    }
    if !is_valid_concept_name(&name) {
        return false;
    }
    let empty_sources: Vec<&str> = Vec::new();
    let params = serde_json::json!({
        "name": name, "display": name, "desc": desc,
        "tags": tags, "sources": empty_sources,
    });
    // KG harvest is a non-blocking side-effect, but a dropped concept on a daemon
    // blip must be observable, not silent — log to the hook stderr channel.
    if let Err(e) = kavach_rpc::client::call::<_, serde_json::Value>("concept.add", Some(params)) {
        use std::io::Write as _;
        drop(writeln!(
            std::io::stderr(),
            "[CONCEPT_ADD_FAIL] {name}: {e}"
        ));
    }
    true
}

/// Concept names: 3–32 chars, lowercase-leading, `[a-z0-9_]`.
fn is_valid_concept_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 3 || bytes.len() > 32 {
        return false;
    }
    if !bytes.first().is_some_and(u8::is_ascii_lowercase) {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
}
