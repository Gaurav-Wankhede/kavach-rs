// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// Deserialize a JSON string that may be explicitly `null` into the default.
/// `#[serde(default)]` only covers a MISSING key; CC v2.1.154 `PreCompact` sends
/// `custom_instructions: null` / `trigger: null` (explicit null), which serde
/// otherwise rejects as "invalid type: null, expected a string".
fn null_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

pub mod gate_config;
pub use gate_config::{
    GateValueDto, gate_enabled, gate_patterns, gate_text, gate_threshold,
};

pub mod six_file;
pub use six_file::{
    ArtifactValidator, AutoDraftSource, FOURTEEN_PREFIXES, MissingPrefix, MissingReason,
    ProjectTier, RequiredPrefix, SpecCategory, SpikeMode, WitnessResult,
};

/// Roadmap/decision ordering weight. Higher number = more urgent (lower sort order).
/// Bounded [0, 1000] so typos (negative, `i64::MAX`) cannot silently reorder the backlog.
///
/// Serializes transparently as the inner i64 (wire/DB compatible with the former
/// Option<i64>). Deserializes from JSON integers back to Priority.
///
/// # Examples
/// ```ignore
/// // Clamping untrusted input (CLI)
/// let p = Priority::new(-5);
/// assert_eq!(p.get(), 0);
///
/// // Strict validation (internal)
/// assert!(Priority::try_new(1000).is_some());
/// assert!(Priority::try_new(1001).is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Priority(i64);

impl Priority {
    pub const MIN: i64 = 0;
    pub const MAX: i64 = 1000;

    /// Clamp into [MIN, MAX] — total, never fails. Use for untrusted input (CLI).
    #[must_use]
    pub fn new(v: i64) -> Self {
        Self(v.clamp(Self::MIN, Self::MAX))
    }

    /// Reject out-of-range instead of clamping. Use where strictness matters.
    #[must_use]
    pub fn try_new(v: i64) -> Option<Self> {
        (Self::MIN..=Self::MAX).contains(&v).then_some(Self(v))
    }

    /// Extract the inner i64 value for storage/comparison.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Active effort level CC attaches to every hook invocation (`CC` 2.1.133+).
///
/// CC sends `{ "level": "low" | "medium" | "high" }`; gates read it to modulate
/// strictness (e.g. relax stop-gate verbosity on `low`, tighten research
/// enforcement on `high`). CC also exports `$CLAUDE_EFFORT` as a fallback.
/// SOURCE: code.claude.com/docs/en/changelog v2.1.133.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO; hook event deserializer; non_exhaustive => E0639"
)]
pub struct EffortInput {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub level: String,
}

/// `HookInput` represents JSON input passed to any hook.
/// Wire-compatible with the Go `types.HookInput` struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO; hook event deserializer; non_exhaustive => E0639"
)]
pub struct HookInput {
    // Common fields - ALL hooks receive these
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
    /// Active effort tier (`CC` 2.1.133+). `None` when CC omits it (older CC or
    /// gateway); gates fall back to `$CLAUDE_EFFORT`. See [`EffortInput`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<EffortInput>,

    // PreToolUse / PostToolUse / PermissionRequest
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_use_id: String,

    // PostToolUse
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<HashMap<String, serde_json::Value>>,

    // UserPromptSubmit
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prompt: String,

    // Stop / SubagentStop
    #[serde(default)]
    pub stop_hook_active: bool,
    /// Background tasks Claude is waiting on (`CC` 2.1.152+). Non-empty means
    /// Claude legitimately cannot stop — e.g. `run_in_background: true` Agent
    /// call is still streaming. Stop hooks MUST yield (`exit_silent`) when this
    /// is populated; otherwise we recreate `GitHub` issue #55754 (~50min loop).
    /// SOURCE: code.claude.com/docs/en/changelog v2.1.152.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub background_tasks: Vec<serde_json::Value>,
    /// Scheduled cron tasks pending in this session (`CC` 2.1.152+). Same yield
    /// semantics as `background_tasks` for the stop-gate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub session_crons: Vec<serde_json::Value>,

    // SubagentStart / SubagentStop
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_transcript_path: String,

    // SessionStart
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,

    // SessionEnd
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,

    // PreCompact — CC v2.1.154 sends these as explicit `null` on bare /compact.
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

    // Notification
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notification_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,

    // Stop / SubagentStop
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_assistant_message: String,

    // InstructionsLoaded / ConfigChange
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub memory_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub load_reason: String,

    // WorktreeCreate
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,

    // WorktreeRemove
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub worktree_path: String,

    // TeammateIdle / TaskCompleted
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub teammate_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub team_name: String,

    // TaskCompleted
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_subject: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_description: String,

    // PostToolUseFailure
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,

    // PostCompact
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub compact_summary: String,

    /// Catch-all for hook-input keys not modelled above. Without this, serde
    /// DROPS unknown fields, so a new in-flight category (e.g. the Monitor tool
    /// shipped `CC` 2026-04-09, whose Stop-hook field name is undocumented) would
    /// be invisible to the stop-gate inflight guard. Retaining the raw map lets
    /// `has_inflight_extra` yield generically on ANY non-empty array category
    /// without hard-coding a field name we cannot yet verify. SOURCE: Monitor
    /// tool — claudefast.com/blog/guide/mechanics/monitor.
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl HookInput {
    /// Return the first `extra` key whose value is a NON-EMPTY JSON array and
    /// whose name signals in-flight work (monitor / task / cron / stream / watch
    /// / active / inflight). Lets the stop-gate yield for in-flight categories
    /// not modelled by a named field — notably Claude Code's Monitor tool, whose
    /// Stop-hook field name is undocumented. Field-name-agnostic by design: it
    /// matches the SIGNAL substrings, not a guessed literal, so it stays correct
    /// across future renames. Returns the matching key for the yield message.
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

    /// Extract a string value from `tool_input` by key.
    /// Falls back to prompt field for "prompt" key.
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

    /// Resolve the active effort tier (`CC` 2.1.133+). Prefers the JSON `effort.level`
    /// field; falls back to the `$CLAUDE_EFFORT` env var CC exports for hooks; returns
    /// `""` when neither is set. Gates key strictness thresholds off this — an empty
    /// string means "no signal", so callers should treat it as the default tier.
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
    /// `SessionStart`: ask CC to re-scan skill directories (`CC` 2.1.152+). Kavach
    /// rebuilds its own skill registry at session start, so it sets this to make CC
    /// pick up any skills installed since launch. SOURCE: changelog v2.1.152.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reload_skills: Option<bool>,
    /// `SessionStart`: set the session's display title (`CC` 2.1.152+). Kavach derives
    /// it from project + dev phase so background/agent views are identifiable.
    /// SOURCE: changelog v2.1.152.
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
    /// Notification: terminal escape sequence CC writes to the controlling tty
    /// (`CC` 2.1.141+) — e.g. a bell (`\x07`) on a permission stall. Empty/`None`
    /// emits nothing. SOURCE: changelog v2.1.141.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_sequence: Option<String>,
}

// --- Constructors ---

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


    // --- Claude Code 2026 format ---

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
    pub fn new_pre_tool_use_modify_input(
        reason: &str,
        updated_input: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "allow".into(),
                permission_decision_reason: reason.into(),
                updated_input: Some(updated_input),
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
            // Stamp the event so a native edge can route this to the harness's
            // stop contract (Cursor's `{continue, followupMessage}`). Claude Code
            // reads only `decision`/`reason` on Stop and ignores this field, so
            // its wire is unchanged.
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "Stop".into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Permission: use decision/reason (no hookSpecificOutput for non-standard events).
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
    pub fn new_permission_allow_with_input(
        reason: &str,
        updated_input: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            decision: "approve".into(),
            reason: reason.into(),
            tool_input: Some(updated_input),
            ..Default::default()
        }
    }

    /// `SessionEnd`: use systemMessage (no hookSpecificOutput).
    #[must_use]
    pub fn new_session_end_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }

    /// `SubagentStart`: use systemMessage (no hookSpecificOutput).
    #[must_use]
    pub fn new_subagent_start_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }

    /// `SubagentStop`: use systemMessage (no hookSpecificOutput).
    #[must_use]
    pub fn new_subagent_stop_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_setup_context(context: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "Setup".into(),
                additional_context: context.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

// TIME: O(0) runtime — expands to () | SPACE: O(0)
// YEAR: 2026 | SEARCHED: 2026-05

/// Zero-cost marker macro. The kavach binary scans source for these
/// invocations and syncs them to kanban as roadmap entries keyed by <file:line>.
///
/// At runtime this expands to a no-op — no impact on user binary.
#[macro_export]
macro_rules! kavach_todo {
    ($desc:literal $(,)?) => {{
        let _ = $desc;
    }};
    ($desc:literal, $($rest:tt)*) => {{
        let _ = $desc;
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflight_extra_detects_monitor_array() {
        // A future/undocumented in-flight category (e.g. the Monitor tool) lands
        // in `extra` via #[serde(flatten)]; a non-empty array under an in-flight-
        // signal key must be detected so the stop-gate yields.
        let json = r#"{"hook_event_name":"Stop","monitors":[{"id":"m1"}]}"#;
        let input: HookInput = serde_json::from_str(json).expect("parse");
        assert_eq!(input.inflight_extra_key(), Some("monitors"));
    }

    #[test]
    fn inflight_extra_ignores_empty_and_benign() {
        // Empty array under a signal key, and a non-empty array under a benign
        // key, must NOT trigger a yield (no false positive).
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
        // Empty JSON level is treated as "no signal" — the accessor falls back to
        // $CLAUDE_EFFORT. We don't mutate the env here (workspace forbids `unsafe`,
        // which `set_var`/`remove_var` require); we only assert the JSON branch is
        // skipped on empty, leaving the env-fallback result (a String, never panics).
        let input = HookInput {
            effort: Some(EffortInput {
                level: String::new(),
            }),
            ..Default::default()
        };
        // Result mirrors $CLAUDE_EFFORT (unset in CI ⇒ ""); the invariant under test
        // is "empty JSON level does not short-circuit to empty-string-from-field".
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
        assert!(!input.is_subagent_event());

        // Round-trip
        let serialized = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_id, "sess_abc");
    }

    #[test]
    fn test_hook_input_empty() {
        let input: HookInput = serde_json::from_str("{}").unwrap();
        assert_eq!(input.get_string("anything"), "");
        assert!(!input.get_bool("anything"));
        assert_eq!(input.get_int("anything"), 0);
    }

    #[test]
    fn test_precompact_null_fields_deserialize() {
        // CC v2.1.154 PreCompact payload sends custom_instructions/trigger as
        // explicit `null` on bare /compact. #[serde(default)] alone rejects this
        // ("invalid type: null, expected a string"); null_string tolerates it.
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

    #[test]
    fn test_subagent_event() {
        let input = HookInput {
            hook_event_name: "SubagentStart".into(),
            ..Default::default()
        };
        assert!(input.is_subagent_event());
    }
}

// MemoryStatus — typed lifecycle states (strum: parse + Display + iter).
// SOURCE: https://docs.rs/strum/0.28 — see decision.types.memory-status-enum.

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
// split: intentional — MemoryStatus is a shared domain value enum (single
// source of truth for CLI + RPC + engine, per the RCA above). It is a plain
// value type in a types library, NOT service orchestrator state; it correctly
// lives in kavach-types/lib.rs.
pub enum MemoryStatus {
    Todo,
    InProgress,
    Done,
    Verified,
}

impl MemoryStatus {
    /// Comma-separated list of all variants in canonical wire form ("todo, `in_progress`, ...").
    /// Used for error messages — replaces hand-rolled `VALID_STATUSES.join`(", ").
    #[must_use]
    pub fn allowed_list() -> String {
        use strum::IntoEnumIterator;
        Self::iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// All variants in canonical order. The single source of truth for any
    /// UI status picker — callers (e.g. the kavach-app editor dropdown) must
    /// iterate this, never a hand-rolled string list (that drift omitted
    /// `planned` and broke the migration).
    #[must_use]
    pub fn all() -> Vec<Self> {
        use strum::IntoEnumIterator;
        Self::iter().collect()
    }

    /// `true` iff a card in this state is DISPATCHABLE to an agent.
    ///
    /// Exactly `Todo` and `InProgress`. `Done` awaits verification and `Verified`
    /// is terminal — neither is runnable. The single typed source for every
    /// dispatch predicate (replaces scattered `matches!(s, "todo" | "in_progress")`
    /// magic-string checks that a typo could silently break at the DB boundary).
    #[must_use]
    pub const fn is_runnable(self) -> bool {
        matches!(self, Self::Todo | Self::InProgress)
    }

    /// `true` iff a card in this state SATISFIES a dependency edge.
    ///
    /// Exactly `Done` and `Verified` — a dependent unblocks once its blocker
    /// reaches either. The typed counterpart to `is_runnable`; the two sets are
    /// disjoint and together partition the four-variant enum.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Done | Self::Verified)
    }
}

#[cfg(test)]
mod memory_status_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parses_canonical_forms() {
        assert_eq!(MemoryStatus::from_str("todo").unwrap(), MemoryStatus::Todo);
        assert_eq!(
            MemoryStatus::from_str("in_progress").unwrap(),
            MemoryStatus::InProgress
        );
    }

    #[test]
    fn rejects_unknown() {
        assert!(MemoryStatus::from_str("garbage").is_err());
    }

    #[test]
    fn display_round_trips() {
        for s in [
            MemoryStatus::Todo,
            MemoryStatus::InProgress,
            MemoryStatus::Done,
            MemoryStatus::Verified,
        ] {
            let rendered = s.to_string();
            let parsed = MemoryStatus::from_str(&rendered).unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn legacy_statuses_rejected() {
        assert!(MemoryStatus::from_str("planned").is_err());
        assert!(MemoryStatus::from_str("blocked").is_err());
        assert!(MemoryStatus::from_str("deferred").is_err());
    }

    #[test]
    fn allowed_list_contains_exactly_canonical_four() {
        let list = MemoryStatus::allowed_list();
        assert!(list.contains("todo"));
        assert!(list.contains("in_progress"));
        assert!(list.contains("done"));
        assert!(list.contains("verified"));
        assert!(!list.contains("planned"));
        assert!(!list.contains("blocked"));
        assert!(!list.contains("deferred"));
    }

    #[test]
    fn runnable_set_is_exactly_todo_and_in_progress() {
        assert!(MemoryStatus::Todo.is_runnable());
        assert!(MemoryStatus::InProgress.is_runnable());
        assert!(!MemoryStatus::Done.is_runnable());
        assert!(!MemoryStatus::Verified.is_runnable());
    }

    #[test]
    fn complete_set_is_exactly_done_and_verified() {
        assert!(MemoryStatus::Done.is_complete());
        assert!(MemoryStatus::Verified.is_complete());
        assert!(!MemoryStatus::Todo.is_complete());
        assert!(!MemoryStatus::InProgress.is_complete());
    }

    #[test]
    fn runnable_and_complete_partition_the_enum_with_no_overlap() {
        // Every variant is in exactly ONE of the two dispatch sets — they are
        // disjoint (no card is both runnable and dependency-satisfying) and total
        // (no variant falls through both predicates).
        for s in MemoryStatus::all() {
            assert!(
                s.is_runnable() ^ s.is_complete(),
                "{s} must be in exactly one of runnable/complete"
            );
        }
    }
}

#[cfg(test)]
mod priority_tests {
    use super::*;

    #[test]
    fn new_clamps_below_min() {
        let p = Priority::new(-5);
        assert_eq!(p.get(), 0);
    }

    #[test]
    fn new_clamps_above_max() {
        let p = Priority::new(2000);
        assert_eq!(p.get(), 1000);
    }

    #[test]
    fn new_accepts_in_range() {
        let p = Priority::new(500);
        assert_eq!(p.get(), 500);
    }

    #[test]
    fn try_new_rejects_below_min() {
        assert!(Priority::try_new(-1).is_none());
    }

    #[test]
    fn try_new_rejects_above_max() {
        assert!(Priority::try_new(1001).is_none());
    }

    #[test]
    fn try_new_accepts_in_range() {
        assert!(Priority::try_new(0).is_some());
        assert!(Priority::try_new(500).is_some());
        assert!(Priority::try_new(1000).is_some());
    }

    #[test]
    fn get_round_trips() {
        let p = Priority::new(42);
        assert_eq!(Priority::new(p.get()), p);
    }

    #[test]
    fn serde_transparent_roundtrip() {
        let p = Priority::new(5);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "5");
        let deserialized: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, p);
    }

    #[test]
    fn serde_in_option() {
        let opt: Option<Priority> = Some(Priority::new(100));
        let json = serde_json::to_string(&opt).unwrap();
        assert_eq!(json, "100");
        let deserialized: Option<Priority> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, opt);
    }

    #[test]
    fn serde_none() {
        let opt: Option<Priority> = None;
        let json = serde_json::to_string(&opt).unwrap();
        assert_eq!(json, "null");
        let deserialized: Option<Priority> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, None);
    }

    #[test]
    fn ordering() {
        let low = Priority::new(1);
        let high = Priority::new(100);
        assert!(low < high);
        assert!(high > low);
        assert_eq!(low, Priority::new(1));
    }

    #[test]
    fn display() {
        let p = Priority::new(42);
        assert_eq!(p.to_string(), "42");
    }
}
