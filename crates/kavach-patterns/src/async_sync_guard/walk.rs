use regex::Regex;
use std::sync::LazyLock;

pub(super) fn walk_fn_bodies(content: &str) -> Vec<(usize, usize, &str, &str, &str)> {
    static FN_RE: LazyLock<Option<Regex>> =
        LazyLock::new(|| Regex::new(r"(?:async\s+)?fn\s+(\w+)").ok());
    let Some(fn_re) = FN_RE.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for m in fn_re.captures_iter(content) {
        let Some(full) = m.get(0) else { continue };
        let Some(name) = m.get(1) else { continue };
        let Some(after) = content.get(full.end()..) else {
            continue;
        };
        let Some(brace_off) = after.find('{') else {
            continue;
        };
        let body_start = full.end().saturating_add(brace_off).saturating_add(1);
        let mut depth = 1usize;
        let mut body_end = body_start;
        let Some(body_slice) = content.get(body_start..) else {
            continue;
        };
        for (i, c) in body_slice.char_indices() {
            match c {
                '{' => depth = depth.saturating_add(1),
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        body_end = body_start.saturating_add(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(before_fn) = content.get(..full.start()) else {
            continue;
        };
        let attrs_start = before_fn.rfind("\n\n").map_or(0, |i| i.saturating_add(2));
        let Some(attrs) = content.get(attrs_start..full.start()) else {
            continue;
        };
        let start_line = before_fn.matches('\n').count().saturating_add(1);
        let Some(before_body_end) = content.get(..body_end) else {
            continue;
        };
        let end_line = before_body_end.matches('\n').count().saturating_add(1);
        let Some(body_content) = content.get(body_start..body_end) else {
            continue;
        };
        out.push((start_line, end_line, name.as_str(), attrs, body_content));
    }
    out
}

pub(super) fn async_fn_line_set(content: &str) -> std::collections::HashSet<usize> {
    let mut set = std::collections::HashSet::with_capacity(64);
    for (start, end, _) in walk_async_fns(content) {
        for ln in start..=end {
            set.insert(ln);
        }
    }
    set
}

pub(super) fn walk_async_fn_bodies(content: &str) -> Vec<(usize, &str)> {
    walk_async_fns(content)
        .into_iter()
        .map(|(s, _, b)| (s, b))
        .collect()
}

pub(super) fn walk_async_fns(content: &str) -> Vec<(usize, usize, &str)> {
    static ASYNC_RE: LazyLock<Option<Regex>> =
        LazyLock::new(|| Regex::new(r"async\s+fn\s+\w+").ok());
    let Some(async_re) = ASYNC_RE.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for m in async_re.find_iter(content) {
        let Some(after) = content.get(m.end()..) else {
            continue;
        };
        let Some(brace_off) = after.find('{') else {
            continue;
        };
        let body_start = m.end().saturating_add(brace_off).saturating_add(1);
        let mut depth = 1usize;
        let mut body_end = body_start;
        let Some(body_slice) = content.get(body_start..) else {
            continue;
        };
        for (i, c) in body_slice.char_indices() {
            match c {
                '{' => depth = depth.saturating_add(1),
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        body_end = body_start.saturating_add(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(before_start) = content.get(..m.start()) else {
            continue;
        };
        let start_line = before_start.matches('\n').count().saturating_add(1);
        let Some(before_body_end) = content.get(..body_end) else {
            continue;
        };
        let end_line = before_body_end.matches('\n').count().saturating_add(1);
        let Some(body_content) = content.get(body_start..body_end) else {
            continue;
        };
        out.push((start_line, end_line, body_content));
    }
    out
}
