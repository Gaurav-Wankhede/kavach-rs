//! Cargo.toml [dependencies]/[dev-dependencies] parser.
use super::extract_major_version;

/// Parse [dependencies] and [dev-dependencies] sections from a Cargo.toml body.
/// Extracts name + major version for each dep. Mutates `result` with new entries.
pub(super) fn parse_cargo_deps(cargo_body: &str, result: &mut Vec<(String, u32)>) {
    let mut in_deps = false;
    for line in cargo_body.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" || trimmed == "[dev-dependencies]" {
            in_deps = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_deps = false;
            continue;
        }
        if !in_deps {
            continue;
        }
        if let Some((name, rest)) = trimmed.split_once('=') {
            let name = name.trim();
            let rest = rest.trim().trim_matches('"');
            // Simple: name = "1.2.3"
            if let Some(major) = extract_major_version(rest)
                && !result.iter().any(|(n, _)| n == name)
            {
                result.push((name.to_owned(), major));
            }
            // Inline table: { version = "1.2" }
            parse_inline_table_version(name, rest, result);
        }
    }
}

/// Handle the `{ version = "1.2" }` inline-table form; updates or inserts.
fn parse_inline_table_version(name: &str, rest: &str, result: &mut Vec<(String, u32)>) {
    let Some(ver_pos) = rest.find("version") else {
        return;
    };
    let Some(after_str) = rest.get(ver_pos..) else {
        return;
    };
    let Some(q1) = after_str.find('"') else {
        return;
    };
    let Some(inner) = after_str.get(q1.saturating_add(1)..) else {
        return;
    };
    if let Some(q2) = inner.find('"')
        && let Some(ver_str) = inner.get(..q2)
        && let Some(major) = extract_major_version(ver_str)
    {
        if let Some(entry) = result.iter_mut().find(|(n, _)| n == name) {
            entry.1 = major;
        } else {
            result.push((name.to_owned(), major));
        }
    }
}
