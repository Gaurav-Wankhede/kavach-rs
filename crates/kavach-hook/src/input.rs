// SOURCE: doc.rust-lang.org/edition-guide/rust-2018/path-changes.html — 2018+ module paths need no mod.rs
use std::io::{self, BufRead};
use kavach_types::HookInput;
/// Read a hook input payload from stdin.
/// # Errors
pub fn read_hook_input() -> Result<HookInput, String> {
    let stdin = io::stdin();
    read_hook_input_from(stdin.lock())
}
/// Read a hook input payload from an arbitrary reader.
/// # Errors
pub fn read_hook_input_from<R: BufRead>(reader: R) -> Result<HookInput, String> {
    let mut buf = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| format!("read error: {e}"))?;
        buf.push(line);
    }
    let raw = buf.join("\n");
    parse_hook_input(&raw)
}
/// Parses raw hook input, scrubbing null fields.
///
/// # Errors
/// Returns `Err` when the payload is not valid JSON or not a JSON object/array.
///
/// # Panics
/// This function does not panic. It handles JSON parsing errors gracefully.
pub fn parse_hook_input(raw: &str) -> Result<HookInput, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    parse_hook_input_from_value(value)
}

/// Parses a pre-deserialized JSON value into a canonical [`HookInput`], scrubbing
/// null fields and tolerating sequence-wrapped objects.
///
/// This entry point lets vendor-specific edges (e.g. Kimi) mutate the raw payload
/// (flatten `ContentPart[]` message arrays into strings) before canonical parsing.
///
/// # Errors
/// Returns `Err` when the payload is not a JSON object/array or the canonical
/// struct cannot be deserialized from it.
pub fn parse_hook_input_from_value(mut value: serde_json::Value) -> Result<HookInput, String> {
    // Handle sequence input by extracting the first object if present.
    if value.is_array() {
        let Some(arr) = value.as_array() else {
            return Err("JSON parse error: array check failed".to_owned());
        };
        if arr.is_empty() {
            return Err("JSON parse error: empty array provided".to_owned());
        }
        // Use the first element if it's an object.
        if let Some(first) = arr.first() {
            if first.is_object() {
                value = first.clone();
            } else {
                return Err(format!(
                    "JSON parse error: array first element is not an object, got {first}"
                ));
            }
        } else {
            return Err("JSON parse error: array has no first element".to_owned());
        }
    }

    if let Some(obj) = value.as_object_mut() {
        obj.retain(|_, v| !v.is_null());
    }
    serde_json::from_value(value).map_err(|e| format!("JSON parse error: {e}"))
}

#[cfg(test)]
#[path = "input_test.rs"]
mod tests;
