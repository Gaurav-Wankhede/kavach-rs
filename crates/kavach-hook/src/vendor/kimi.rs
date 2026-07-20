// split: Kimi Code native edge. Kimi mirrors the Claude Code hook contract (same
// PascalCase event names, snake_case stdin JSON), so input lowering is the
// canonical null-tolerant parse; the ONE native difference is the exit-code-2
// block signal on output (config format — TOML [[hooks]] — is install-side only).
//! Kimi Code CLI native edge.
//!
//! Kimi deliberately mirrors Claude Code's hook contract.
//!
//! Identical event names (`PreToolUse`, `Stop`, `UserPromptSubmit`…), snake_case
//! stdin JSON (`hook_event_name`, `session_id`, `tool_input`…), and the same
//! `hookSpecificOutput.permissionDecision` blocking JSON. The ONE native
//! divergence is the OUTPUT failure signal: Kimi treats hook **exit code 2** as a
//! hard block, with stderr as the reason (other non-zero exits fail OPEN).
//! SOURCE: <https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html>

use kavach_types::HookInput;

/// Lower a raw Kimi payload into the canonical [`HookInput`].
///
/// Kimi deliberately mirrors the Claude Code hook contract, but has a few wire
/// divergences. We normalize them here so the canonical gates see the same
/// shape they expect from Claude Code:
///
/// - `prompt` may arrive as a `ContentPart[]` array; text parts are flattened
///   to a space-joined string. `response` (SubagentStop) is also flattened and
///   mapped to `prompt` when no canonical prompt is present.
/// - `body` is renamed to `message` (Notification event).
/// - `agent_name` is copied to `agent_id` and `agent_type` (SubagentStart/
///   SubagentStop), since Kimi only supplies the name and the subagent gates
///   track by id and classify by type.
/// - `error_message` is renamed to `error` (StopFailure).
/// - Flat tool fields (`file_path`, `content`, `command`, `old_string`,
///   `new_string`) and their aliases (`path`, `contents`) are lifted into
///   `tool_input` so the pre-write and pre-tool gates see the canonical nested
///   shape. SOURCE: same issue as Cursor before `cursor_tool_input`.
///
/// Any extra Kimi fields are ignored by the canonical struct without error.
/// (A Kimi payload is shape-indistinguishable from Claude Code, so payload
/// sniffing cannot detect it; the installed hook pins `--vendor kimi`.)
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all.
pub fn lower(raw_payload: &str) -> Result<HookInput, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw_payload).map_err(|e| format!("JSON parse error: {e}"))?;

    let Some(obj) = value.as_object_mut() else {
        return crate::parse_hook_input_from_value(value);
    };

    // Kimi sends message-like fields as ContentPart[] arrays. Flatten text
    // parts to a plain string so the canonical HookInput deserializer
    // (which expects String) does not fail with:
    //   "invalid type: sequence, expected a string".
    // SOURCE: https://github.com/MoonshotAI/kimi-code/issues/917
    if let Some(v) = obj.get_mut("prompt")
        && let Some(parts) = v.as_array()
    {
        *v = serde_json::Value::String(flatten_content_parts(parts));
    }
    // SubagentStop carries the worker's final output in `response`; map it to
    // the canonical `prompt` field so downstream context injection can surface
    // it, but only when a canonical prompt is not already present.
    if let Some(v) = obj.get_mut("response")
        && let Some(parts) = v.as_array()
    {
        *v = serde_json::Value::String(flatten_content_parts(parts));
    }
    if !obj.contains_key("prompt") {
        rename_field(obj, "response", "prompt");
    }

    // Normalize Kimi-native field names onto the canonical HookInput fields.
    // These are no-ops when the canonical field is already present.
    rename_field(obj, "body", "message");
    copy_field_if_absent(obj, "agent_name", "agent_id");
    copy_field_if_absent(obj, "agent_name", "agent_type");
    rename_field(obj, "error_message", "error");

    // Kimi may send ANY text-bearing field as a ContentPart[] array (not only
    // `prompt`/`response`). Recursively flatten them before canonical struct
    // deserialization, which expects String fields.
    flatten_content_part_fields(obj);

    // Lift flat tool fields into tool_input so every gate reads the canonical
    // nested shape. Without this, a PreToolUse payload that carries file_path at
    // the top level causes the pre-write gate to fail-closed with PATH_POLICY.
    lift_flat_tool_fields(obj);

    crate::parse_hook_input_from_value(value)
}

/// Ensure `tool_input` contains every security-critical tool field, copying them
/// from the top level when a vendor sends them flat. Mirrors Cursor's
/// `cursor_tool_input` for the same PATH_POLICY/WRITE_BYPASS root cause.
fn lift_flat_tool_fields(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let mut tool_input = obj
        .get("tool_input")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    for key in ["file_path", "content", "command", "old_string", "new_string"] {
        if tool_input.contains_key(key) {
            continue;
        }
        if let Some(v) = obj.get(key).filter(|v| !v.is_null()) {
            tool_input.insert(key.to_owned(), v.clone());
        }
    }
    // Vendor aliases: path -> file_path, contents -> content.
    for (native, canonical) in [("path", "file_path"), ("contents", "content")] {
        if tool_input.contains_key(canonical) {
            continue;
        }
        let source = tool_input
            .get(native)
            .cloned()
            .or_else(|| obj.get(native).filter(|v| !v.is_null()).cloned());
        if let Some(v) = source {
            tool_input.insert(canonical.to_owned(), v);
        }
    }

    if !tool_input.is_empty() {
        obj.insert("tool_input".to_owned(), serde_json::Value::Object(tool_input));
    }
}

/// Extract text from a Kimi `ContentPart[]` array, joining multiple text parts
/// with a single space and ignoring non-text parts.
fn flatten_content_parts(parts: &[serde_json::Value]) -> String {
    parts
        .iter()
        .filter_map(|part| {
            if part.get("type")?.as_str()? == "text" {
                part.get("text")?.as_str().map(str::to_owned)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// True if `v` is a Kimi `ContentPart[]` array.
fn is_content_part_array(v: &serde_json::Value) -> bool {
    v.as_array().is_some_and(|arr| {
        arr.iter().all(|part| {
            part.is_object()
                && part
                    .as_object()
                    .is_some_and(|o| o.contains_key("type") || o.contains_key("text"))
        })
    })
}

/// True if `v` is an array of plain strings.
fn is_string_array(v: &serde_json::Value) -> bool {
    v.as_array()
        .is_some_and(|arr| arr.iter().all(|part| part.is_string()))
}

/// Extract text from a plain string array, joining elements with a single space.
fn flatten_string_array(parts: &[serde_json::Value]) -> String {
    parts
        .iter()
        .filter_map(|part| part.as_str().map(str::to_owned))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Recursively flatten message-bearing arrays into plain strings.
/// Handles both Kimi `ContentPart[]` arrays and plain `string[]` arrays.
/// Operates on object values and on nested `tool_input` objects.
/// Legitimate list fields (`background_tasks`, `session_crons`) are left intact.
fn flatten_content_part_fields(obj: &mut serde_json::Map<String, serde_json::Value>) {
    const SKIP_KEYS: &[&str] = &["background_tasks", "session_crons"];
    for (k, v) in obj.iter_mut() {
        if SKIP_KEYS.contains(&k.as_str()) {
            continue;
        }
        let flat = if is_content_part_array(v) {
            v.as_array().map(|parts| flatten_content_parts(parts))
        } else if is_string_array(v) {
            v.as_array().map(|parts| flatten_string_array(parts))
        } else {
            if let Some(nested) = v.as_object_mut() {
                flatten_content_part_fields(nested);
            }
            None
        };
        if let Some(text) = flat {
            *v = serde_json::Value::String(text);
        }
    }
}

/// Rename a JSON object field, preserving the original if the target already
/// exists (canonical wins).
fn rename_field(obj: &mut serde_json::Map<String, serde_json::Value>, from: &str, to: &str) {
    if obj.contains_key(to) {
        return;
    }
    if let Some(v) = obj.remove(from) {
        obj.insert(to.to_owned(), v);
    }
}

/// Copy a JSON object field to another key if the target is absent and the
/// source is present. Unlike [`rename_field`], the source is kept in place.
fn copy_field_if_absent(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    from: &str,
    to: &str,
) {
    if obj.contains_key(to) {
        return;
    }
    if let Some(v) = obj.get(from) {
        obj.insert(to.to_owned(), v.clone());
    }
}

/// Render a canonical verdict into Kimi's native output.
///
/// The body is the same Claude-Code JSON Kimi understands; the exit-code-2 block
/// signal is applied by the caller via [`super::Vendor::block_exit_code`].
#[must_use]
pub fn render(resp: &kavach_types::HookResponse) -> String {
    serde_json::to_string(resp)
        .unwrap_or_else(|_| r#"{"decision":"block","reason":"hook internal error"}"#.to_owned())
}
