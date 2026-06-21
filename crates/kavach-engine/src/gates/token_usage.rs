//! Extract latest token usage from transcript tail window for spend tracking.
//! See decision.engine.token-usage-tail-window for design rationale.

/// Sum of the most recent `usage` block in the transcript JSONL tail, or
/// `None` if the path is empty/unreadable or no usage line is present.
///
/// Claude Code transcript assistant lines carry
/// `message.usage.{input_tokens,output_tokens,cache_creation_input_tokens,
/// cache_read_input_tokens}`. All four are summed so cached-prompt spend is
/// counted toward the budget (cache reads still cost). The LAST usage line in
/// the tail is the freshest turn — scan forward, keep the last match.
pub(super) fn extract_latest_token_usage(transcript_path: &str) -> Option<i32> {
    use std::io::{Read, Seek, SeekFrom};
    if transcript_path.is_empty() {
        return None;
    }
    let path = std::path::Path::new(transcript_path);
    let len = std::fs::metadata(path).ok()?.len();
    let window: u64 = 64 * 1024;
    let offset = len.saturating_sub(window);
    let mut file = std::fs::File::open(path).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = String::new();
    file.take(window).read_to_string(&mut buf).ok()?;

    let mut latest: Option<i32> = None;
    for line in buf.lines() {
        if !line.contains("\"usage\"") {
            continue;
        }
        // Truncated first line of the tail window fails to parse — skip it
        // (bounded-window invariant: window may start mid-record).
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        // usage nests under message.usage on assistant lines; tolerate a
        // top-level `usage` too.
        let Some(usage) = v
            .get("message")
            .and_then(|m| m.get("usage"))
            .or_else(|| v.get("usage"))
        else {
            continue;
        };
        let sum: i64 = [
            "input_tokens",
            "output_tokens",
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
        ]
        .iter()
        .filter_map(|k| usage.get(*k).and_then(serde_json::Value::as_i64))
        .sum();
        if sum > 0 {
            // A single turn never exceeds i32; saturate explicitly rather
            // than truncate so an absurd value can't wrap negative.
            latest = Some(i32::try_from(sum).unwrap_or(i32::MAX));
        }
    }
    latest
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_transcript(lines: &[&str]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "kavach-tok-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let mut f = std::fs::File::create(&p).expect("create tmp transcript");
        for l in lines {
            writeln!(f, "{l}").expect("write line");
        }
        p
    }

    #[test]
    fn none_for_empty_path() {
        assert_eq!(extract_latest_token_usage(""), None);
    }

    #[test]
    fn none_for_missing_file() {
        assert_eq!(
            extract_latest_token_usage("/no/such/transcript.jsonl"),
            None
        );
    }

    #[test]
    fn sums_usage_fields_of_latest_line() {
        let p = tmp_transcript(&[
            r#"{"type":"assistant","message":{"usage":{"input_tokens":10,"output_tokens":5}}}"#,
            r#"{"type":"user","content":"hi"}"#,
            r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":20,"cache_read_input_tokens":7}}}"#,
        ]);
        // Latest usage line: 100 + 20 + 7 = 127.
        assert_eq!(extract_latest_token_usage(&p.to_string_lossy()), Some(127));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn none_when_no_usage_lines() {
        let p = tmp_transcript(&[r#"{"type":"user","content":"hi"}"#]);
        assert_eq!(extract_latest_token_usage(&p.to_string_lossy()), None);
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn tolerates_truncated_leading_line() {
        // First line is a JSON fragment (tail window cut mid-record) yet
        // still contains the substring "usage"; must skip it on parse
        // failure and still read the valid later usage line.
        let p = tmp_transcript(&[
            r#"_usage":999}}}  <-- garbage fragment"#,
            r#"{"type":"assistant","message":{"usage":{"input_tokens":3,"output_tokens":4}}}"#,
        ]);
        assert_eq!(extract_latest_token_usage(&p.to_string_lossy()), Some(7));
        std::fs::remove_file(&p).ok();
    }
}
