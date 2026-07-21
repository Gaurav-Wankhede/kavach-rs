use std::collections::HashSet;

const DEFAULT_MAX_TOKENS: usize = 2_000;
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
            if remaining < 100 { break; }
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
