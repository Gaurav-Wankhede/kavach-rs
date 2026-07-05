//! Single choke for gate injection: compress + fire-and-forget metric record.
use kavach_toon::compact::{compress, Level};

/// Compress `text` at Full level, fire-and-forget a rot-savings metric, return the text.
pub fn compact_inject(text: &str) -> String {
    let out = compress(text, Level::Full);
    record_metric(text, &out);
    out
}

fn record_metric(input: &str, output: &str) {
    let tokens_in = input.split_whitespace().count();
    let tokens_out = output.split_whitespace().count();
    let delta = tokens_in as i64 - tokens_out as i64;
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
    let sw = kavach_session::SpooledWrite::new("db.write".to_string(), params_json);
    if let Err(e) = kavach_session::enqueue_write_spool(&sw) {
        eprintln!("kavach: compact metric spool enqueue failed: {e}");
    }
}

#[cfg(test)]
#[path = "inject_test.rs"]
mod inject_tests;
