use crate::EffortInput;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn null_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

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
        // 1. Canonical shape: nested under tool_input.
        if let Some(v) = self.tool_input_string(key) {
            return v;
        }
        // 2. Top-level struct field fallback (e.g., flat file_path on some vendors).
        if let Some(v) = self.top_level_string(key) {
            return v;
        }
        // 3. Fields captured by #[serde(flatten)] extra when a vendor sends them flat.
        if let Some(v) = self.extra_string(key) {
            return v;
        }
        // 4. Vendor aliases: path -> file_path, contents -> content.
        if let Some(alias) = vendor_alias(key) {
            if let Some(v) = self.tool_input_string(alias) {
                return v;
            }
            if let Some(v) = self.extra_string(alias) {
                return v;
            }
        }
        ""
    }

    fn tool_input_string(&self, key: &str) -> Option<&str> {
        self.tool_input
            .as_ref()
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    }

    fn extra_string(&self, key: &str) -> Option<&str> {
        self.extra
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    }

    fn top_level_string(&self, key: &str) -> Option<&str> {
        let s = match key {
            "file_path" => &self.file_path,
            "session_id" => &self.session_id,
            "transcript_path" => &self.transcript_path,
            "cwd" => &self.cwd,
            "permission_mode" => &self.permission_mode,
            "hook_event_name" => &self.hook_event_name,
            "tool_name" => &self.tool_name,
            "tool_use_id" => &self.tool_use_id,
            "message" => &self.message,
            "notification_type" => &self.notification_type,
            "title" => &self.title,
            "agent_id" => &self.agent_id,
            "agent_type" => &self.agent_type,
            "source" => &self.source,
            "model" => &self.model,
            "reason" => &self.reason,
            "trigger" => &self.trigger,
            "custom_instructions" => &self.custom_instructions,
            "name" => &self.name,
            "worktree_path" => &self.worktree_path,
            "teammate_name" => &self.teammate_name,
            "team_name" => &self.team_name,
            "task_id" => &self.task_id,
            "task_subject" => &self.task_subject,
            "task_description" => &self.task_description,
            "error" => &self.error,
            "compact_summary" => &self.compact_summary,
            _ => return None,
        };
        if s.is_empty() { None } else { Some(s) }
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

fn vendor_alias(key: &str) -> Option<&'static str> {
    match key {
        "file_path" => Some("path"),
        "content" => Some("contents"),
        _ => None,
    }
}
