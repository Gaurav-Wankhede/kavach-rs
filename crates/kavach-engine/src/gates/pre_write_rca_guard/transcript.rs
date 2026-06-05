//! Transcript JSONL scanner: look for an `[RCA]` block (or durable `rca.` decision
//! row) in any assistant record of the in-flight Claude Code conversation.
use super::detect::{has_rca_block, line_persists_rca_decision};

/// 8 MiB ceiling: far above any real recent-turn transcript window, yet bounds
/// worst-case allocation. Beyond it we read a line-aligned tail (dropping the
/// partial leading record) — the only case where head-truncation is unavoidable.
const CEILING: u64 = 8 * 1024 * 1024;

/// Scan the in-flight Claude Code transcript JSONL for an `[RCA]` block in any
/// assistant message.
///
/// # Errors
/// FIX `contract_violation`: `PreToolUse` payload does NOT include
/// `last_assistant_message` (`kavach-types/src/lib.rs:74` — `Stop`/`SubagentStop` only).
/// `transcript_path` IS populated on `PreToolUse` and contains the in-flight
/// conversation up to the pending tool call.
pub(in crate::gates) fn scan_transcript_for_rca(transcript_path: &str) -> bool {
    if transcript_path.is_empty() {
        return false;
    }
    let path = std::path::Path::new(transcript_path);
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let Some(buf) = read_complete_records(path, metadata.len()) else {
        return false;
    };
    buf.lines().any(|line| {
        (line.contains("\"role\":\"assistant\"") || line.contains("\"type\":\"assistant\""))
            && (has_rca_block(line) || line_persists_rca_decision(line))
    })
}

/// Read the transcript as COMPLETE JSONL records, not a raw byte tail.
///
/// Claude Code records are newline-delimited, but a single assistant/tool record
/// in a long turn is frequently >>32 KB. A fixed byte window slices such a record
/// on both ends and is STRUCTURALLY incapable of seeing an `[RCA]` token in a
/// record whose start lies before the offset (the false-positive that blocked 3
/// valid prose RCAs). So read the whole file, bounded by [`CEILING`], trimmed to
/// a record boundary so every scanned line is a complete record.
fn read_complete_records(path: &std::path::Path, len: u64) -> Option<String> {
    if len <= CEILING {
        return std::fs::read_to_string(path).ok();
    }
    let raw = std::fs::read(path).ok()?;
    // Drop the partial leading record so every remaining line is complete.
    let start = raw
        .iter()
        .position(|&b| b == b'\n')
        .map_or(raw.len(), |nl| nl.saturating_add(1));
    raw.get(start..)
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
}
