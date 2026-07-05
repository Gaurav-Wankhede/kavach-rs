//! Single choke for gate injection: compress + fire-and-forget metric record.
// SOURCE: anthropic.com/engineering/effective-context-engineering-for-ai-agents
use kavach_toon::compact::{compress, Level};

/// Compress `text` at Ultra level, fire-and-forget a rot-savings metric, return the text.
#[must_use]
pub fn compact_inject(text: &str) -> String {
    let out = compress(text, Level::Ultra);
    record_metric(text, &out);
    out
}

fn record_metric(input: &str, output: &str) {
    let tokens_in = input.split_whitespace().count();
    let tokens_out = output.split_whitespace().count();
    let delta = i64::try_from(tokens_in).unwrap_or(i64::MAX).saturating_sub(
        i64::try_from(tokens_out).unwrap_or(i64::MAX),
    );
    let session_id = kavach_session::resolved_session_id();
    if session_id.is_empty() {
        return;
    }
    // No session row yet means no known project; skip rather than write a garbage key.
    let Some(state) = kavach_session::load_session_state_for(&session_id) else {
        return;
    };
    let params = serde_json::json!({
        "project": state.project,
        "category": "pattern",
        "key": format!("compact.metric.{session_id}"),
        "title": "Compact compression metrics (session)",
        "content": format!("last_delta_tok={delta} tok_in={tokens_in} tok_out={tokens_out}"),
        "new": true,
    });
    let Ok(params_json) = serde_json::to_string(&params) else {
        return;
    };
    let sw = kavach_session::SpooledWrite::new("db.write".to_owned(), params_json);
    // Fire-and-forget: an enqueue failure is best-effort lost, never blocks the gate.
    drop(kavach_session::enqueue_write_spool(&sw));
}

#[cfg(test)]
#[path = "inject_test.rs"]
mod inject_tests;
