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

mod priority;
pub use priority::Priority;

mod effort_input;
pub use effort_input::EffortInput;

mod hook_io;
pub use hook_io::{HookInput, HookResponse, HookSpecificOutput};

mod memory_status;
pub use memory_status::MemoryStatus;




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

        // Round-trip
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
