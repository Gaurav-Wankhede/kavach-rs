/// Extract preserved spans, longest-first so fences/URLs mask before sub-parts.
pub(super) fn preserved_spans(text: &str) -> Vec<String> {
    let mut spans: Vec<String> = Vec::new();
    spans.extend(fenced_blocks(text));
    spans.extend(inline_code(text));
    spans.extend(urls(text));
    spans.extend(file_line_tokens(text));
    spans.extend(bracket_tokens(text));
    spans.extend(version_tokens(text));
    spans.sort_by_key(|s| std::cmp::Reverse(s.len()));
    spans.dedup();
    spans
}

fn fenced_blocks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("```") {
        let Some(after_open) = rest.get(start.saturating_add(3)..) else {
            break;
        };
        let Some(end) = after_open.find("```") else {
            break;
        };
        let end_abs = start.saturating_add(3).saturating_add(end).saturating_add(3);
        let Some(block) = rest.get(start..end_abs) else {
            break;
        };
        out.push(block.to_owned());
        let Some(next) = rest.get(end_abs..) else {
            break;
        };
        rest = next;
    }
    out
}

fn inline_code(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'`'
            && let Some(rel_end) = text.get(i.saturating_add(1)..).and_then(|t| t.find('`'))
        {
            let end = i.saturating_add(1).saturating_add(rel_end).saturating_add(1);
            if let Some(span) = text.get(i..end) {
                out.push(span.to_owned());
            }
            i = end;
            continue;
        }
        i = i.saturating_add(1);
    }
    out
}

fn urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for scheme in ["https://", "http://"] {
        let mut consumed = 0usize;
        while let Some(rel) = text.get(consumed..).and_then(|t| t.find(scheme)) {
            let abs = consumed.saturating_add(rel);
            let Some(tail) = text.get(abs..) else { break };
            let len = tail.find(char::is_whitespace).unwrap_or(tail.len());
            if let Some(span) = tail.get(..len) {
                out.push(span.to_owned());
            }
            consumed = abs.saturating_add(len);
        }
    }
    out
}

fn file_line_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '/' && c != '.' && c != ':' && c != '_' && c != '-'
        });
        if trimmed.contains('/')
            && trimmed.contains(':')
            && let Some((path, line)) = trimmed.rsplit_once(':')
            && !path.is_empty()
            && !line.is_empty()
            && line.chars().all(|c| c.is_ascii_digit())
        {
            out.push(trimmed.to_owned());
        }
    }
    out
}

fn bracket_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut consumed = 0usize;
    while let Some(rel_open) = text.get(consumed..).and_then(|t| t.find('[')) {
        let abs_open = consumed.saturating_add(rel_open);
        let Some(tail) = text.get(abs_open.saturating_add(1)..) else {
            break;
        };
        if let Some(rel_close) = tail.find(']') {
            if let Some(inner) = tail.get(..rel_close)
                && !inner.is_empty()
                && inner.chars().all(|c| c.is_ascii_uppercase() || c == '_')
            {
                let abs_close = abs_open
                    .saturating_add(1)
                    .saturating_add(rel_close)
                    .saturating_add(1);
                if let Some(span) = text.get(abs_open..abs_close) {
                    out.push(span.to_owned());
                }
            }
            consumed = abs_open
                .saturating_add(1)
                .saturating_add(rel_close)
                .saturating_add(1);
        } else {
            consumed = abs_open.saturating_add(1);
        }
    }
    out
}

fn version_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if trimmed.contains('.') && trimmed.chars().all(|c| c.is_ascii_digit() || c == '.') {
            let parts: Vec<&str> = trimmed.split('.').collect();
            if parts.len() >= 2
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            {
                out.push(trimmed.to_owned());
            }
        }
    }
    out
}

const SENTINEL: char = '\u{0}';

/// Replace each preserved span with a sentinel placeholder, longest-first.
pub(super) fn mask(text: &str, spans: &[String]) -> (String, Vec<String>) {
    let mut masked = text.to_owned();
    let mut originals = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(pos) = masked.find(span.as_str()) {
            let idx = originals.len();
            let placeholder = format!("{SENTINEL}{idx}{SENTINEL}");
            masked.replace_range(pos..pos.saturating_add(span.len()), &placeholder);
            originals.push(span.clone());
        }
    }
    (masked, originals)
}

/// Restore sentinel placeholders to their original preserved spans.
pub(super) fn unmask(masked: &str, originals: &[String]) -> String {
    let mut out = masked.to_owned();
    for (idx, original) in originals.iter().enumerate() {
        let placeholder = format!("{SENTINEL}{idx}{SENTINEL}");
        out = out.replace(&placeholder, original);
    }
    out
}
