//! In-process translation from legacy POSIX commands to Rust toolbelt equivalents.

#[cfg(test)]
#[path = "toolbelt_rewrite_test.rs"]
mod tests;

/// Mapping of legacy POSIX tools to their Rust toolbelt equivalents.
const TOOLBELT_MAPPINGS: &[(&str, &str)] = &[
    ("grep", "rg"),
    ("find", "fd"),
    ("cat", "bat --paging=never"),
    ("ls", "eza"),
    ("tree", "erd"),
    ("sed", "sd"),
    ("jq", "jaq"),
    ("curl", "xh"),
    ("diff", "difft"),
    ("du", "dust"),
    ("ps", "procs"),
];

/// Rewrite command segments matching legacy POSIX tools into Rust toolbelt equivalents.
#[must_use]
pub(crate) fn rewrite_toolbelt(cmd: &str) -> Option<String> {
    let segments = split_pipeline(cmd);
    let mut changed = false;
    let mut reconstructed = String::with_capacity(cmd.len().saturating_add(16));

    for (segment, sep) in segments {
        if let Some(rewritten) = rewrite_single_toolbelt(segment) {
            changed = true;
            reconstructed.push_str(&rewritten);
        } else {
            reconstructed.push_str(segment);
        }
        if let Some(s) = sep {
            reconstructed.push_str(s);
        }
    }

    changed.then_some(reconstructed)
}

/// Rewrite a single sub-command word if it matches a legacy POSIX tool.
#[must_use]
pub(crate) fn rewrite_single_toolbelt(cmd: &str) -> Option<String> {
    let trimmed = cmd.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let leading_ws = cmd.get(..cmd.len().saturating_sub(trimmed.len())).unwrap_or("");

    let (env_prefix, command_body) = extract_env_prefix(trimmed);

    for &(legacy, replacement) in TOOLBELT_MAPPINGS {
        if let Some(rest) = command_body.strip_prefix(legacy)
            && (rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            return Some(format!("{leading_ws}{env_prefix}{replacement}{rest}"));
        }
    }

    None
}

/// Extract leading KEY=VAL variable assignments preceding a command word.
fn extract_env_prefix(s: &str) -> (&str, &str) {
    let mut offset = 0;
    for part in s.split_whitespace() {
        if let Some((k, _)) = part.split_once('=')
            && !k.is_empty()
            && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && let Some(pos) = s.get(offset..).and_then(|sub| sub.find(part))
        {
            offset = offset.saturating_add(pos).saturating_add(part.len());
            while s.get(offset..).is_some_and(|sub| sub.starts_with(char::is_whitespace)) {
                offset = offset.saturating_add(1);
            }
            continue;
        }
        break;
    }
    let prefix = s.get(..offset).unwrap_or("");
    let body = s.get(offset..).unwrap_or(s);
    (prefix, body)
}

/// Quote-aware splitting of pipeline / compound command separators (&&, ||, |, ;).
pub(crate) fn split_pipeline(cmd: &str) -> Vec<(&str, Option<&str>)> {
    let mut segments = Vec::new();
    let mut quote: Option<char> = None;
    let mut prev_backslash = false;
    let mut seg_start = 0;
    let chars: Vec<(usize, char)> = cmd.char_indices().collect();
    let mut i = 0;

    while i < chars.len() {
        let (idx, ch) = chars.get(i).copied().unwrap_or((cmd.len(), ' '));

        if let Some(q) = quote {
            if ch == q && !prev_backslash {
                quote = None;
            }
            prev_backslash = ch == '\\' && !prev_backslash;
            i = i.saturating_add(1);
            continue;
        }

        if (ch == '\'' || ch == '"') && !prev_backslash {
            quote = Some(ch);
            i = i.saturating_add(1);
            continue;
        }

        prev_backslash = ch == '\\' && !prev_backslash;

        // Check 2-character operators: && or ||
        if let (Some(&(_, ch1)), Some(&(idx2, ch2))) = (chars.get(i), chars.get(i.saturating_add(1)))
            && ((ch1 == '&' && ch2 == '&') || (ch1 == '|' && ch2 == '|'))
        {
            let sep_end = idx2.saturating_add(1);
            let segment = cmd.get(seg_start..idx).unwrap_or("");
            let sep = cmd.get(idx..sep_end);
            segments.push((segment, sep));
            seg_start = sep_end;
            i = i.saturating_add(2);
            continue;
        }

        // Check 1-character operators: | or ;
        if ch == '|' || ch == ';' {
            let sep_end = idx.saturating_add(1);
            let segment = cmd.get(seg_start..idx).unwrap_or("");
            let sep = cmd.get(idx..sep_end);
            segments.push((segment, sep));
            seg_start = sep_end;
            i = i.saturating_add(1);
            continue;
        }

        i = i.saturating_add(1);
    }

    if seg_start <= cmd.len() {
        let rem = cmd.get(seg_start..).unwrap_or("");
        segments.push((rem, None));
    }

    segments
}
