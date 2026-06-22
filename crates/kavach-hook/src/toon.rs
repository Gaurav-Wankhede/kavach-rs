// Compact serializer for additionalContext payloads.
// ARCH: Compact key-value format reduces tokens vs verbose JSON.
// PATTERN: fallback-only | SCOPE: context serialization | SEARCHED: 2026-04
// NOTE: toon-format dependency HELD due to 58 unvetted transitive deps.
//       Using compact fallback format until TOON is actively needed.

/// Encode a slice of key-value pairs as compact format.
/// Format: "key: value" per line — compact and human-readable.
#[must_use]
pub fn encode_kvs(kvs: &[(&str, &str)]) -> String {
    fallback_kvs(kvs)
}

/// Encode an array of uniform objects as TOON tabular format.
///
/// Format: `[N]{field1,field2}:` header (keys declared once) + one
/// comma-separated value row per object — the header-dedup is the token win.
/// SOURCE: <https://github.com/toon-format/toon> — uniform-array tabular grammar.
/// Non-uniform rows (differing key sets) fall back to the plain per-row form,
/// which the spec also prefers for ragged data.
#[must_use]
pub fn encode_table(rows: &[Vec<(&str, &str)>]) -> String {
    encode_table_named("", rows)
}

/// Named variant: emits `name[N]{fields}:` so the frame is self-describing.
/// Empty `name` omits the name token (`[N]{fields}:`).
#[must_use]
pub fn encode_table_named(name: &str, rows: &[Vec<(&str, &str)>]) -> String {
    let Some(first) = rows.first() else {
        return String::new();
    };
    let fields: Vec<&str> = first.iter().map(|(k, _)| *k).collect();
    if !rows.iter().all(|r| row_matches_fields(r, &fields)) {
        return fallback_table(rows); // ragged — plain per-row form per spec
    }
    let header = format!(
        "{name}[{n}]{{{fields}}}:",
        n = rows.len(),
        fields = fields.join(",")
    );
    let body = rows
        .iter()
        .map(|r| r.iter().map(|(_, v)| toon_cell(v)).join(","))
        .join("\n  ");
    format!("{header}\n  {body}")
}

/// A row matches when its keys equal `fields` in order — the uniformity TOON requires.
fn row_matches_fields(row: &[(&str, &str)], fields: &[&str]) -> bool {
    row.len() == fields.len() && row.iter().zip(fields).all(|((k, _), f)| k == f)
}

/// CSV-style cell: quote when the value holds a comma, newline, or quote;
/// escape embedded quotes by doubling. SOURCE: TOON spec quoting (CSV-like).
fn toon_cell(v: &str) -> String {
    if v.contains([',', '\n', '"']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_owned()
    }
}

/// Encode a list of strings as comma-separated values.
#[must_use]
pub fn encode_list(items: &[&str]) -> String {
    items.join(", ")
}

/// Fallback: plain key-value format if TOON encoding fails.
/// SOURCE: <https://docs.rs/itertools/0.13/itertools/trait.Itertools.html#method.join>
/// `itertools::join` writes Display values directly into the output buffer, skipping
/// the intermediate Vec<String> that .`collect::`<Vec<_>>().`join()` would allocate.
fn fallback_kvs(kvs: &[(&str, &str)]) -> String {
    use itertools::Itertools as _;
    kvs.iter().map(|(k, v)| format!("{k}: {v}")).join("\n")
}

/// Fallback: simple table format if TOON encoding fails.
fn fallback_table(rows: &[Vec<(&str, &str)>]) -> String {
    use itertools::Itertools as _;
    rows.iter()
        .map(|row| row.iter().map(|(k, v)| format!("{k}={v}")).join(" "))
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests follow Rust 2026 best practices:
    // - Descriptive names (should_X_when_Y pattern)
    // - Test edge cases (empty, single, multiple)
    // - Independent tests (no shared state)
    // Source: doc.rust-lang.org/book/ch11-03-test-organization.html

    #[test]
    fn should_encode_kvs_with_colon_format() {
        let kvs = &[("status", "allow"), ("gate", "pre_write")];
        let result = encode_kvs(kvs);
        assert!(
            result.contains("status: allow"),
            "kvs uses 'key: value' format"
        );
        assert!(result.contains("gate: pre_write"));
    }

    #[test]
    fn should_handle_empty_kvs_without_panic() {
        let kvs: &[(&str, &str)] = &[];
        let result = encode_kvs(kvs);
        assert!(result.is_empty(), "empty kvs produces empty string");
    }

    #[test]
    fn should_preserve_kv_values_in_output() {
        let kvs = &[("key", "value123"), ("another", "test456")];
        let result = encode_kvs(kvs);
        assert!(result.contains("value123"), "value should be preserved");
        assert!(result.contains("test456"), "all values should be preserved");
    }

    #[test]
    fn should_encode_table_as_toon_tabular_with_deduped_header() {
        let rows = vec![
            vec![("skill", "rust"), ("priority", "P0")],
            vec![("skill", "data"), ("priority", "P1")],
        ];
        let result = encode_table(&rows);
        assert!(
            result.starts_with("[2]{skill,priority}:"),
            "header declares count + fields once: {result}"
        );
        assert!(result.contains("rust,P0"), "value row is comma-separated");
        assert!(result.contains("data,P1"));
        assert!(
            !result.contains("skill=data"),
            "keys are NOT repeated per row — that is the token win"
        );
    }

    #[test]
    fn should_quote_cells_containing_commas() {
        let rows = vec![vec![("title", "a, b"), ("k", "plain")]];
        let result = encode_table(&rows);
        assert!(result.contains("\"a, b\",plain"), "comma value quoted: {result}");
    }

    #[test]
    fn should_double_embedded_quotes() {
        let rows = vec![vec![("k", "he said \"hi\"")]];
        let result = encode_table(&rows);
        assert!(result.contains("\"he said \"\"hi\"\"\""), "quotes doubled: {result}");
    }

    #[test]
    fn should_fall_back_for_ragged_rows() {
        let rows = vec![
            vec![("a", "1"), ("b", "2")],
            vec![("a", "1")], // different key set
        ];
        let result = encode_table(&rows);
        assert!(!result.contains("[2]{"), "ragged rows must not claim uniform header");
        assert!(result.contains("a=1"), "falls back to per-row form: {result}");
    }

    #[test]
    fn toon_tabular_beats_per_row_on_tokens() {
        // The whole point: header-dedup shrinks the byte/token count vs repeating keys.
        let rows = vec![
            vec![("status", "verified"), ("key", "k1"), ("title", "t1")],
            vec![("status", "todo"), ("key", "k2"), ("title", "t2")],
            vec![("status", "done"), ("key", "k3"), ("title", "t3")],
        ];
        let toon = encode_table(&rows);
        let per_row = fallback_table(&rows);
        assert!(
            toon.len() < per_row.len(),
            "TOON {} bytes must be < per-row {} bytes",
            toon.len(),
            per_row.len()
        );
    }

    #[test]
    fn should_name_table_when_provided() {
        let rows = vec![vec![("k", "v")]];
        assert!(encode_table_named("cards", &rows).starts_with("cards[1]{k}:"));
    }

    #[test]
    fn should_handle_empty_table_without_panic() {
        let rows: Vec<Vec<(&str, &str)>> = vec![];
        let result = encode_table(&rows);
        assert!(result.is_empty(), "empty table produces empty string");
    }

    #[test]
    fn should_handle_single_row_table() {
        let rows = vec![vec![("only", "row")]];
        let result = encode_table(&rows);
        assert_eq!(result, "[1]{only}:\n  row", "single row encodes as TOON tabular");
    }

    #[test]
    fn should_encode_list_as_comma_separated() {
        let items = &["rust", "data", "web-stack"];
        let result = encode_list(items);
        assert_eq!(result, "rust, data, web-stack");
    }

    #[test]
    fn should_handle_empty_list_without_panic() {
        let items: &[&str] = &[];
        let result = encode_list(items);
        assert!(result.is_empty(), "empty list produces empty string");
    }

    #[test]
    fn should_handle_single_item_list() {
        let items = &["only"];
        let result = encode_list(items);
        assert_eq!(result, "only", "single item should be encoded");
    }

    #[test]
    fn should_format_fallback_kvs_with_colon_separator() {
        let kvs = &[("a", "1"), ("b", "2")];
        let result = fallback_kvs(kvs);
        assert!(result.contains("a: 1"), "fallback uses 'key: value' format");
        assert!(result.contains("b: 2"));
    }

    #[test]
    fn should_format_fallback_table_with_equals_separator() {
        let rows = vec![vec![("x", "10"), ("y", "20")]];
        let result = fallback_table(&rows);
        assert!(result.contains("x=10"), "fallback uses 'key=value' format");
        assert!(result.contains("y=20"));
    }

    #[test]
    fn should_handle_special_characters_in_values() {
        let kvs = &[("msg", "hello world"), ("path", "/usr/bin")];
        let result = encode_kvs(kvs);
        assert!(result.contains("hello world"));
        assert!(result.contains("/usr/bin"));
    }

    // SOURCE: https://docs.rs/insta/1.40 — snapshot testing locks output shape.
    // Run `cargo insta review` to accept format changes intentionally.
    #[test]
    fn snapshot_kvs_format() {
        let kvs = &[
            ("status", "allow"),
            ("gate", "pre_write"),
            ("priority", "P0"),
        ];
        insta::assert_snapshot!(encode_kvs(kvs), @r###"
        status: allow
        gate: pre_write
        priority: P0
        "###);
    }

    #[test]
    fn snapshot_table_format() {
        let rows = vec![
            vec![("skill", "rust"), ("priority", "P0")],
            vec![("skill", "data"), ("priority", "P1")],
        ];
        insta::assert_snapshot!(encode_table(&rows), @r###"
        [2]{skill,priority}:
          rust,P0
          data,P1
        "###);
    }

    #[test]
    fn snapshot_list_format() {
        let items = &["rust", "data", "web-stack"];
        insta::assert_snapshot!(encode_list(items), @"rust, data, web-stack");
    }
}
