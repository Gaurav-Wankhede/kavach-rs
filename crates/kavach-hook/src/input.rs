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
/// Parses raw hook input, scrubbing null fields. Errors if not a JSON object.
pub fn parse_hook_input(raw: &str) -> Result<HookInput, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    if let Some(obj) = value.as_object_mut() {
        obj.retain(|_, v| !v.is_null());
    }
    serde_json::from_value(value).map_err(|e| format!("JSON parse error: {e}"))
}
