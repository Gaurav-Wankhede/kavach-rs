use std::io::Read as _;

use kavach_toon::compact::{self, Level};
use serde_json::json;

/// `kavach compact` — compress stdin with the deployed compact compressor (debug/witness).
pub(super) fn run(level: &str, verify: bool, record: bool, project: Option<&str>) -> i32 {
    let level_str = level;
    let level = match level.to_ascii_lowercase().as_str() {
        "lite" => Level::Lite,
        "full" => Level::Full,
        "ultra" => Level::Ultra,
        other => {
            eprintln!("kavach compact: unknown --level '{other}' (expected lite|full|ultra)");
            return 2;
        }
    };

    let mut input = String::new();
    let read_result = std::io::stdin().lock().read_to_string(&mut input);
    if let Err(e) = read_result {
        eprintln!("kavach compact: failed to read stdin: {e}");
        return 2;
    }

    // SOURCE: anthropic.com/engineering/effective-context-engineering-for-ai-agents
    let output = compact::compress(&input, level);
    println!("{output}");

    if verify && let Err(e) = compact::assert_lossless(&input, &output) {
        eprintln!("kavach compact: lossless check failed: {e}");
        return 1;
    }

    if record {
        return record_metrics(level_str, &input, &output, project);
    }

    0
}

/// Measure the compression Δ, verify losslessness, and persist a metric row via `db.write`.
fn record_metrics(level_str: &str, input: &str, output: &str, project: Option<&str>) -> i32 {
    let Some(proj) = project else {
        eprintln!("kavach compact: --record requires --project (no session resolver available)");
        return 2;
    };

    let bytes_in = input.len();
    let bytes_out = output.len();
    let tokens_in = input.split_whitespace().count();
    let tokens_out = output.split_whitespace().count();
    let delta_pct = if tokens_in > 0 {
        let tok_in_i = i64::try_from(tokens_in).unwrap_or(i64::MAX);
        let tok_out_i = i64::try_from(tokens_out).unwrap_or(i64::MAX);
        tok_out_i
            .saturating_sub(tok_in_i)
            .saturating_mul(100)
            .checked_div(tok_in_i)
            .unwrap_or(0)
    } else {
        0
    };
    let lossless_ok = compact::assert_lossless(input, output).is_ok();

    let key = format!("compact.run.{proj}.{tokens_in}x{tokens_out}");
    let title = format!("Compact compression metrics ({proj})");
    let content = format!(
        "level={level_str} bytes_in={bytes_in} bytes_out={bytes_out} tok_in={tokens_in} tok_out={tokens_out} delta_pct={delta_pct} lossless={lossless_ok}"
    );
    let params = json!({
        "project": proj,
        "category": "pattern",
        "key": key,
        "title": title,
        "content": content,
        "new": true,
    });

    if let Err(e) =
        kavach_rpc::client::call::<serde_json::Value, serde_json::Value>("db.write", Some(params))
    {
        eprintln!("kavach compact: rpc db.write: {e}");
        return 3;
    }

    eprintln!(
        "[COMPACTION_RECORDED] {level_str} {tokens_in}->{tokens_out} tok ({delta_pct}%) lossless={lossless_ok} row={proj}/pattern/{key}"
    );
    0
}
