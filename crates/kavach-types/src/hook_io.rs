use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use crate::EffortInput;

fn null_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

/// `HookInput` represents JSON input passed to any hook.
/// Wire-compatible with the Go `types.HookInput` struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO; hook event deserializer; non_exhaustive => E0639"
)]
pub struct HookInput {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub transcript_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cwd: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub permission_mode: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hook_event_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<EffortInput>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_use_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<HashMap<String, serde_json::Value>>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prompt: String,

    #[serde(default)]
    pub stop_hook_active: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub background_tasks: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub session_crons: Vec<serde_json::Value>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_transcript_path: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,

    #[serde(
        default,
        deserialize_with = "null_string",
        skip_serializing_if = "String::is_empty"
    )]
    pub trigger: String,
    #[serde(
        default,
        deserialize_with = "null_string",
        skip_serializing_if = "String::is_empty"
    )]
    pub custom_instructions: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notification_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_assistant_message: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub memory_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub load_reason: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub worktree_path: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub teammate_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub team_name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_subject: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_description: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub compact_summary: String,

    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl HookInput {
    #[must_use]
    pub fn inflight_extra_key(&self) -> Option<&str> {
        const SIGNALS: [&str; 7] = [
            "monitor", "task", "cron", "stream", "watch", "active", "inflight",
        ];
        self.extra.iter().find_map(|(k, v)| {
            let key_lc = k.to_lowercase();
            let is_inflight_name = SIGNALS.iter().any(|s| key_lc.contains(s));
            let is_nonempty_array = v.as_array().is_some_and(|a| !a.is_empty());
            (is_inflight_name && is_nonempty_array).then_some(k.as_str())
        })
    }

    #[must_use]
    pub fn get_string(&self, key: &str) -> &str {
        if key == "prompt" && !self.prompt.is_empty() {
            return &self.prompt;
        }
        self.tool_input
            .as_ref()
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    #[must_use]
    pub fn is_event(&self, event: &str) -> bool {
        self.hook_event_name == event
    }

    #[must_use]
    pub fn effort_level(&self) -> String {
        if let Some(e) = self.effort.as_ref()
            && !e.level.is_empty()
        {
            return e.level.clone();
        }
        std::env::var("CLAUDE_EFFORT").unwrap_or_default()
    }
}

/// `HookSpecificOutput` provides structured output per hook event type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO; hook response builder; non_exhaustive => E0639"
)]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub permission_decision: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub permission_decision_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub additional_context: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reload_skills: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_title: Option<String>,
}

/// `HookResponse` represents the hook's decision output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO; hook constructors + callers; non_exhaustive => E0639"
)]
pub struct HookResponse {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub decision: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub additional_context: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<HashMap<String, serde_json::Value>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#continue: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stop_reason: String,
    #[serde(default)]
    pub suppress_output: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub system_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_sequence: Option<String>,
}

impl HookResponse {
    #[must_use]
    pub fn new_approve(reason: &str) -> Self {
        Self {
            decision: "approve".into(),
            reason: reason.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_block(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_modify(reason: &str, context: &str) -> Self {
        Self {
            decision: "approve".into(),
            reason: reason.into(),
            additional_context: context.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_allow(reason: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "allow".into(),
                permission_decision_reason: reason.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_deny(reason: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "deny".into(),
                permission_decision_reason: reason.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_ask(reason: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "ask".into(),
                permission_decision_reason: reason.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_with_context(reason: &str, context: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "allow".into(),
                permission_decision_reason: reason.into(),
                additional_context: context.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_post_tool_use_block(reason: &str, context: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PostToolUse".into(),
                additional_context: context.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_user_prompt_submit_context(context: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "UserPromptSubmit".into(),
                additional_context: context.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_user_prompt_submit_block(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "UserPromptSubmit".into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_stop_block(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "Stop".into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_permission_allow(reason: &str) -> Self {
        Self {
            decision: "approve".into(),
            reason: reason.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_permission_deny(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_session_end_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_subagent_start_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_subagent_stop_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflight_extra_detects_monitor_array() {
        let json = r#"{"hook_event_name":"Stop","monitors":[{"id":"m1"}]}"#;
        let input: HookInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.inflight_extra_key(), Some("monitors"));
    }

    #[test]
    fn inflight_extra_ignores_empty_and_benign() {
        let json = r#"{"monitors":[],"tags":["a","b"],"note":"x"}"#;
        let input: HookInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.inflight_extra_key(), None);
    }

    #[test]
    fn effort_level_prefers_json_field() {
        let input = HookInput {
            effort: Some(EffortInput {
                level: "high".into(),
            }),
            ..Default::default()
        };
        assert_eq!(input.effort_level(), "high");
    }

    #[test]
    fn effort_level_empty_json_falls_through_to_env() {
        let input = HookInput {
            effort: Some(EffortInput {
                level: String::new(),
            }),
            ..Default::default()
        };
        assert_eq!(
            input.effort_level(),
            std::env::var("CLAUDE_EFFORT").unwrap_or_default()
        );
    }

    #[test]
    fn effort_deserializes_from_cc_wire_shape() {
        let input: HookInput =
            serde_json::from_str(r#"{"hook_event_name":"Stop","effort":{"level":"low"}}"#).unwrap();
        assert_eq!(input.effort_level(), "low");
    }

    #[test]
    fn test_hook_input_serde_roundtrip() {
        let json = r#"{
            "session_id": "sess_abc",
            "tool_name": "Bash",
            "tool_input": {"command": "ls -la"},
            "hook_event_name": "PreToolUse",
            "cwd": "/tmp"
        }"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.session_id, "sess_abc");
        assert_eq!(input.tool_name, "Bash");
        assert_eq!(input.get_string("command"), "ls -la");
        assert!(input.is_event("PreToolUse"));

        let serialized = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_id, "sess_abc");
    }

    #[test]
    fn test_hook_input_empty() {
        let input: HookInput = serde_json::from_str("{}").unwrap();
        assert_eq!(input.get_string("anything"), "");
    }

    #[test]
    fn test_precompact_null_fields_deserialize() {
        let json = r#"{
            "session_id": "sess_x",
            "hook_event_name": "PreCompact",
            "trigger": null,
            "custom_instructions": null
        }"#;
        let input: HookInput =
            serde_json::from_str(json).expect("explicit-null PreCompact must parse");
        assert_eq!(input.trigger, "");
        assert_eq!(input.custom_instructions, "");
    }

    #[test]
    fn test_hook_input_prompt_fallback() {
        let input = HookInput {
            prompt: "hello world".into(),
            ..Default::default()
        };
        assert_eq!(input.get_string("prompt"), "hello world");
    }

    #[test]
    fn test_hook_response_approve() {
        let resp = HookResponse::new_approve("ok");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""decision":"approve"#));
        assert!(json.contains(r#""reason":"ok"#));
    }

    #[test]
    fn test_hook_response_block() {
        let resp = HookResponse::new_block("denied");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""decision":"block"#));
    }

    #[test]
    fn test_hook_specific_output_pretooluse() {
        let resp = HookResponse::new_pre_tool_use_deny("blocked cmd");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("PreToolUse"));
        assert!(json.contains("deny"));
    }

    #[test]
    fn test_hook_response_roundtrip_legacy() {
        let resp = HookResponse::new_modify("gate", "context here");
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: HookResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.decision, "approve");
        assert_eq!(parsed.additional_context, "context here");
    }
}
