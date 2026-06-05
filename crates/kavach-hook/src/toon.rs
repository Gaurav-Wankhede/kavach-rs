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

/// Encode an array of uniform objects as compact table format.
/// Format: "key=value key2=value2" per row — space-separated pairs.
#[must_use]
pub fn encode_table(rows: &[Vec<(&str, &str)>]) -> String {
    fallback_table(rows)
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
    fn should_encode_table_with_equals_format() {
        let rows = vec![
            vec![("skill", "rust"), ("priority", "P0")],
            vec![("skill", "data"), ("priority", "P1")],
        ];
        let result = encode_table(&rows);
        assert!(
            result.contains("skill=rust"),
            "table uses 'key=value' format"
        );
        assert!(result.contains("priority=P0"));
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
        assert!(result.contains("only=row"), "single row should be encoded");
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
        skill=rust priority=P0
        skill=data priority=P1
        "###);
    }

    #[test]
    fn snapshot_list_format() {
        let items = &["rust", "data", "web-stack"];
        insta::assert_snapshot!(encode_list(items), @"rust, data, web-stack");
    }
}
