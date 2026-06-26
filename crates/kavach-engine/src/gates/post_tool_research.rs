// TIME: O(n) | SPACE: O(k)
// YEAR: 2024 | SEARCHED: 2026-05
// SOURCE: https://github.com/ashvardanian/StringWars
#![expect(
    clippy::arithmetic_side_effects,
    reason = "bounded byte-offset scanner: all arithmetic is index math over a fixed input byte slice, guarded by .len() bounds and the 6-digit cap; no overflow path"
)]

use kavach_types::HookInput;
use memchr::memmem;

use crate::error::EngineError;

const HARVEST_CAP: usize = 5;
const RFC_NEEDLE: &str = "RFC ";

/// Handle research done: mark research, inject context, fire-and-forget harvest.
///
/// # Errors
/// Returns `Ok(())` on every path; the `Result` matches the `post_tool::run`
/// match dispatch so all per-tool handlers share one return type.
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature fixed by the post_tool::run match dispatch: every per-tool handler returns Result<(), EngineError>"
)]
pub(crate) fn handle(
    input: &HookInput,
    session: &mut kavach_session::SessionState,
) -> Result<(), EngineError> {
    let query = input.get_string("query");

    if query.is_empty() {
        session.mark_research_done();
    } else {
        session.mark_research_done_with_topic(query);
    }
    session.record_websearch();

    let result_text = input
        .tool_response
        .as_ref()
        .and_then(|r| r.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    harvest_concepts(query, result_text);
    store_nlm_doc(query, result_text);

    let context = kavach_hook::context_block("POST_TOOL:RESEARCH", &[]);
    drop(kavach_hook::exit_post_tool_context(&context));
    Ok(())
}

fn harvest_concepts(query: &str, result_text: &str) {
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    scan_rfc(query.as_bytes(), &mut seen);
    scan_rfc(result_text.as_bytes(), &mut seen);
    scan_capitalized(result_text, &mut seen);
    for name in seen.into_iter().take(HARVEST_CAP) {
        let empty_sources: Vec<&str> = Vec::new();
        let params = serde_json::json!({
            "name": name, "display": name,
            "desc": "auto-harvested from WebSearch",
            "tags": ["auto-harvested"], "sources": empty_sources,
        });
        // Non-blocking KG enrichment, but a dropped harvest on a daemon blip must
        // be observable — log to stderr rather than swallow it silently.
        if let Err(e) =
            kavach_rpc::client::call::<_, serde_json::Value>("concept.add", Some(params))
        {
            use std::io::Write as _;
            drop(writeln!(
                std::io::stderr(),
                "[CONCEPT_HARVEST_FAIL] {name}: {e}"
            ));
        }
    }
}

/// Store the research result into the NLM BM25 doc corpus so future prompts can
/// retrieve it (`nlm.query`). Closes the NLM WRITE path (store was registered but
/// had no producer). Fail-soft-but-observable: a daemon blip logs to stderr, never
/// blocks. WebSearch has no single URL, so provenance is the `websearch:<query>`
/// scheme. See decision.engine.nlm-store-wired-from-research.
fn store_nlm_doc(query: &str, result_text: &str) {
    if result_text.trim().is_empty() {
        return;
    }
    let source_url = if query.is_empty() {
        "websearch:unscoped".to_owned()
    } else {
        format!("websearch:{query}")
    };
    // Epoch-ms provenance stamp (engine idiom: SystemTime, not chrono — see post_write.rs).
    let captured_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default();
    let params = serde_json::json!({
        "source_url": source_url,
        "heading": query,
        "body": result_text,
        "captured_at": captured_at,
    });
    if let Err(e) = kavach_rpc::client::call::<_, serde_json::Value>("nlm.store", Some(params)) {
        use std::io::Write as _;
        drop(writeln!(
            std::io::stderr(),
            "[NLM_STORE_FAIL] {source_url}: {e}"
        ));
    }
}

fn scan_rfc(bytes: &[u8], into: &mut std::collections::BTreeSet<String>) {
    let finder = memmem::Finder::new(RFC_NEEDLE);
    for pos in finder.find_iter(bytes) {
        let start = pos + RFC_NEEDLE.len();
        let mut end = start;
        while end - start < 6 && bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        // SOURCE: https://rust-lang.github.io/rust-clippy/rust-1.95.0/index.html#collapsible_if
        if end > start
            && let Some(slice) = bytes.get(start..end)
            && let Ok(num) = std::str::from_utf8(slice)
        {
            into.insert(format!("rfc_{num}"));
        }
    }
}

fn scan_capitalized(text: &str, into: &mut std::collections::BTreeSet<String>) {
    for word in text.split_whitespace() {
        let trimmed: String = word
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if trimmed.len() < 3 || trimmed.len() > 32 {
            continue;
        }
        let upper = trimmed.chars().filter(char::is_ascii_uppercase).count();
        if upper >= 2 && trimmed.chars().any(|c| c.is_ascii_lowercase()) {
            into.insert(trimmed.to_lowercase());
            if into.len() >= 64 {
                return;
            }
        }
    }
}
