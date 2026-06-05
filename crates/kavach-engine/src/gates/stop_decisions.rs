//! Auto-extraction of `[RCA]`/`[DESIGN]`/`[CRATE_DECISION]`/`[ARCH]` blocks from
//! the agent's final response into the kavach-db decision store (extracted from
//! stop.rs). Pure scanner + a thin RPC writer; capped per turn to bound writes.

const DECISION_BLOCK_CAP: usize = 4;
const DECISION_MARKERS: &[&str] = &["[RCA]", "[DESIGN]", "[CRATE_DECISION]", "[ARCH]"];

/// Scan `chat_text` for decision-marker blocks and persist up to
/// `DECISION_BLOCK_CAP` of them as `decision` rows. A block runs from its marker
/// line to the next marker or a blank line.
pub(crate) fn scan_decision_blocks(chat_text: &str, project: &str, turn: i64) {
    let mut written = 0;
    let mut current: Option<(String, String)> = None;
    for line in chat_text.lines() {
        if written >= DECISION_BLOCK_CAP {
            return;
        }
        let trimmed = line.trim_start();
        if let Some(kind) = detect_decision_marker(trimmed) {
            if let Some((k, body)) = current.take()
                && !body.is_empty()
            {
                write_auto_decision(&k, &body, project, turn);
                written = written.saturating_add(1);
            }
            current = Some((kind, String::new()));
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            if line.trim().is_empty() {
                if let Some((k, b)) = current.take()
                    && !b.is_empty()
                {
                    write_auto_decision(&k, &b, project, turn);
                    written = written.saturating_add(1);
                }
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    if let Some((k, body)) = current.take()
        && !body.is_empty()
        && written < DECISION_BLOCK_CAP
    {
        write_auto_decision(&k, &body, project, turn);
    }
}

fn detect_decision_marker(line: &str) -> Option<String> {
    for m in DECISION_MARKERS {
        if line.starts_with(m) {
            return Some(m.trim_matches(['[', ']']).to_owned());
        }
    }
    None
}

fn write_auto_decision(kind: &str, body: &str, project: &str, turn: i64) {
    let kind_lower = kind.to_lowercase();
    let hash = blake3::hash(body.as_bytes());
    let hash8: String = hash.to_hex().chars().take(8).collect();
    let key = format!("auto.{kind_lower}.{turn}.{hash8}");
    let title = format!("auto-extracted {kind} block (turn {turn})");
    let params = serde_json::json!({
        "project": project,
        "category": "decision",
        "key": key,
        "title": title,
        "content": body,
        "new": true,
        "update_key": serde_json::Value::Null,
        "priority": serde_json::Value::Null,
    });
    drop(kavach_rpc::client::call::<_, serde_json::Value>(
        "db.write",
        Some(params),
    ));
}

#[cfg(test)]
mod tests {
    use super::detect_decision_marker;

    #[test]
    fn detects_known_markers_at_line_start() {
        assert_eq!(detect_decision_marker("[RCA]").as_deref(), Some("RCA"));
        assert_eq!(
            detect_decision_marker("[DESIGN] x").as_deref(),
            Some("DESIGN")
        );
        assert_eq!(
            detect_decision_marker("[CRATE_DECISION]").as_deref(),
            Some("CRATE_DECISION")
        );
        assert_eq!(detect_decision_marker("[ARCH]").as_deref(), Some("ARCH"));
    }

    #[test]
    fn ignores_non_markers_and_mid_line_mentions() {
        assert!(detect_decision_marker("plain text").is_none());
        assert!(detect_decision_marker("see [RCA] above").is_none());
        assert!(detect_decision_marker("[UNKNOWN]").is_none());
    }
}
