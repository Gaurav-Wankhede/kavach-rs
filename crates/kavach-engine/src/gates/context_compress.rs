use std::collections::HashSet;

const DEFAULT_MAX_TOKENS: usize = 1_200;#[expect(dead_code)]const HARD_CAP_TOKENS: usize = 2_000;
const CHARS_PER_TOKEN: usize = 4;

pub(crate) fn compress(context: &str, max_tokens: Option<usize>) -> String {
    let max = max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
    let max_chars = max * CHARS_PER_TOKEN;
    if context.len() <= max_chars {
        return context.to_owned();
    }
    let sections = parse_sections(context);
    let scored: Vec<(Section, u8)> = sections.into_iter().map(|s| (s.clone(), score_section(&s))).collect();
    let mut output = String::with_capacity(max_chars);
    let mut used = 0usize;
    for (section, score) in &scored {
        if *score >= 90 {
            let needed = section.content.len();
            if used + needed > max_chars { break; }
            output.push_str(&section.header);
            output.push_str(&section.content);
            used += needed + section.header.len();
        }
    }
    for (section, score) in &scored {
        if *score >= 70 && *score < 90 {
            let remaining = max_chars.saturating_sub(used);
            if remaining == 0 { break; }
            let truncated = truncate_to_budget(&section.content, remaining);
            output.push_str(&section.header);
            output.push_str(&truncated);
            used += truncated.len() + section.header.len();
        }
    }
    for (section, score) in &scored {
        if *score >= 40 && *score < 70 {
            let remaining = max_chars.saturating_sub(used);
            if remaining < 200 { break; }
            let truncated = truncate_to_budget(&section.content, remaining);
            output.push_str(&section.header);
            output.push_str(&truncated);
            used += truncated.len() + section.header.len();
        }
    }
    if used < context.len() {
        output.push_str(&format!("\n[COMPRESSED] {}B -> {}B ({} tokens saved)\n", context.len(), used, (context.len() - used) / CHARS_PER_TOKEN));
    }
    output
}

#[derive(Clone)]
struct Section { header: String, content: String }

fn parse_sections(context: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current_header = String::new();
    let mut current_content = String::new();
    for line in context.lines() {
        if line.starts_with('[') && line.contains(']') {
            if !current_header.is_empty() {
                sections.push(Section { header: current_header.clone(), content: current_content.clone() });
            }
            current_header = format!("{line}\n");
            current_content.clear();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    if !current_header.is_empty() {
        sections.push(Section { header: current_header, content: current_content });
    } else if !current_content.is_empty() {
        sections.push(Section { header: String::new(), content: current_content });
    }
    sections
}

fn score_section(section: &Section) -> u8 {
    let h = section.header.to_ascii_lowercase();
    if h.contains("[autonomy_contract]") || h.contains("[memory_guard]") || h.contains("[reconcile]") || h.contains("[case_facts") { return 100; }
    if h.contains("[kanban]") || h.contains("[intent]") || h.contains("[session_start]") || h.contains("[pre_compact]") || h.contains("[post_compact]") { return 80; }
    if h.contains("[practice_delta]") || h.contains("[decision_map]") || h.contains("[pattern_dag]") || h.contains("[carry_forward]") || h.contains("[research:") { return 60; }
    if h.contains("[hot_patterns]") || h.contains("[mistake_ledger]") || h.contains("[learned_policy]") || h.contains("[flow]") || h.contains("[stack_fit]") || h.contains("[kavach_lld]") || h.contains("[zero_grep_tools]") { return 30; }
    50
}

fn truncate_to_budget(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes { return content.to_owned(); }
    let mut out = String::with_capacity(max_bytes);
    let mut used = 0usize;
    for ch in content.chars() {
        let next = used.saturating_add(ch.len_utf8());
        if next > max_bytes.saturating_sub(20) { break; }
        out.push(ch);
        used = next;
    }
    out.push_str("\n…[truncated]\n");
    out
}

pub(crate) fn deduplicate_lines(context: &str) -> String {
    let mut seen = HashSet::new();
    let mut output = String::with_capacity(context.len());
    for line in context.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { output.push('\n'); continue; }
        if trimmed.starts_with('[') || trimmed.starts_with("---") {
            output.push_str(line); output.push('\n'); continue;
        }
        if seen.insert(trimmed.to_owned()) { output.push_str(line); output.push('\n'); }
    }
    output
}

pub(crate) fn compress_hook_context(context: &str) -> String {
    let deduped = deduplicate_lines(context);
    compress(&deduped, None)
}

// SOURCE: kavach decision.context-rot-surrealdb-pipeline
#[expect(dead_code, reason = "wired in session_start/memory/query.rs and patterns/mistakes/graph.rs")]
const DB_NOISE_KEYS: &[&str] = &["id", "created_at", "updated_at", "_id", "_key", "_meta", "timestamp", "session_id", "project_id", "owner", "permissions"];

#[expect(dead_code, reason = "wired in session_start/memory/query.rs")]
pub(crate) fn compress_db_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if DB_NOISE_KEYS.contains(&k.as_str()) { continue; }
                out.insert(k.clone(), compress_db_json(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(compress_db_json).collect()),
        _ => value.clone(),
    }
}

#[expect(dead_code, reason = "wired in RPC response handlers")]
pub(crate) fn compress_db_json_string(json_str: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return json_str.to_owned(),
    };
    let compressed = compress_db_json(&parsed);
    serde_json::to_string(&compressed).unwrap_or_else(|_| json_str.to_owned())
}

#[expect(dead_code, reason = "wired in session_start/memory/query.rs")]
pub(crate) fn compress_db_rows(rows: &[serde_json::Value], max_rows: usize) -> Vec<serde_json::Value> {
    let capped = if rows.len() > max_rows { &rows[..max_rows] } else { rows };
    capped.iter().map(|r| compress_db_json(r)).collect()
}
const DB_NOISE_KEYS: &[&str] = &["id", "created_at", "updated_at", "_id", "_key", "_meta", "timestamp", "session_id", "project_id", "owner", "permissions"];

pub(crate) fn compress_db_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if DB_NOISE_KEYS.contains(&k.as_str()) { continue; }
                out.insert(k.clone(), compress_db_json(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(compress_db_json).collect()),
        _ => value.clone(),
    }
}

pub(crate) fn compress_db_json_string(json_str: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return json_str.to_owned(),
    };
    let compressed = compress_db_json(&parsed);
    serde_json::to_string(&compressed).unwrap_or_else(|_| json_str.to_owned())
}

pub(crate) fn compress_db_rows(rows: &[serde_json::Value], max_rows: usize) -> Vec<serde_json::Value> {
    let capped = if rows.len() > max_rows { &rows[..max_rows] } else { rows };
    capped.iter().map(|r| compress_db_json(r)).collect()
}
