// hub: multi-harness native edge — vendor detection + per-vendor input lowering
// and output rendering. The ONLY place in the tree that knows a non-Claude-Code
// harness exists; the engine/gates/DB stay vendor-blind (decision.multi-harness-native-edges).
//! Native edges for the three harnesses kavach runs inside — Claude Code,
//! Cursor, Codex.
//!
//! Each harness spawns the same kavach binary and pipes JSON over stdin/stdout,
//! but in its OWN dialect. This module is the anti-corruption layer: it detects
//! which harness called (hybrid: explicit override wins, else payload sniff),
//! LOWERS that harness's native input into the canonical [`HookInput`] pivot the
//! gates reason over, and RENDERS the canonical [`HookResponse`] verdict back
//! into that harness's native output contract — including its native failure
//! policy (Cursor fails OPEN; Codex/Claude Code fail closed).
//!
//! SOURCES: <https://cursor.com/docs/hooks> · <https://developers.openai.com/codex/hooks>

use kavach_types::{HookInput, HookResponse};

pub mod codex;
pub mod cursor;

#[cfg(test)]
#[path = "vendor_test.rs"]
mod tests;

/// Which harness invoked kavach. `ClaudeCode` is the canonical dialect (the pivot
/// IS its native shape) and the safest-compatible default for an unknown payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Vendor {
    /// Anthropic Claude Code — canonical hook contract.
    #[default]
    ClaudeCode,
    /// Cursor IDE — `conversation_id`/`workspace_roots`, camelCase events,
    /// `{continue,permission,userMessage}` output, fail-OPEN on error.
    Cursor,
    /// `OpenAI` Codex CLI — Claude-Code-compatible events plus `turn_id` /
    /// `permission_mode`; blocks via exit code 2.
    Codex,
}

/// The env var that force-selects a vendor, overriding payload auto-detect.
pub const VENDOR_ENV: &str = "KAVACH_HARNESS";

impl Vendor {
    /// Parse a vendor tag (CLI `--vendor` value or `KAVACH_HARNESS`), case-insensitive.
    /// `None` for an empty/unknown tag so the caller falls through to auto-detect.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag.trim().to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" | "cc" => Some(Self::ClaudeCode),
            "cursor" => Some(Self::Cursor),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    /// Resolve the vendor for one invocation — the hybrid policy:
    /// 1. `explicit` (CLI `--vendor`) if it names a known vendor,
    /// 2. else `$KAVACH_HARNESS`,
    /// 3. else auto-detect from the payload shape,
    /// 4. else [`Vendor::ClaudeCode`] (safest-compatible default).
    #[must_use]
    pub fn resolve(explicit: Option<&str>, raw_payload: &str) -> Self {
        if let Some(v) = explicit.and_then(Self::from_tag) {
            return v;
        }
        if let Some(v) = std::env::var(VENDOR_ENV)
            .ok()
            .and_then(|t| Self::from_tag(&t))
        {
            return v;
        }
        Self::detect(raw_payload)
    }

    /// Auto-detect the vendor from the raw payload's shape. Cursor and Codex each
    /// carry signature fields a Claude Code payload never does; anything else is
    /// treated as Claude Code (the canonical, most-compatible default).
    #[must_use]
    pub fn detect(raw_payload: &str) -> Self {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw_payload) else {
            return Self::ClaudeCode;
        };
        let has = |k: &str| v.get(k).is_some_and(|x| !x.is_null());
        // Cursor: conversation_id + workspace_roots are unique to its payload.
        if has("conversation_id") || has("workspace_roots") || has("generation_id") {
            return Self::Cursor;
        }
        // Codex: turn_id is its turn-scope extension over the CC contract.
        if has("turn_id") {
            return Self::Codex;
        }
        Self::ClaudeCode
    }

    /// Lower a raw native payload into the canonical [`HookInput`] pivot. Every
    /// vendor path is null-tolerant (the W1 invariant) and maps native field
    /// names + event names onto the canonical ones.
    ///
    /// # Errors
    /// Returns `Err` only when the payload is not a JSON object at all.
    pub fn lower(self, raw_payload: &str) -> Result<HookInput, String> {
        match self {
            Self::ClaudeCode => crate::parse_hook_input(raw_payload),
            Self::Cursor => cursor::lower(raw_payload),
            Self::Codex => codex::lower(raw_payload),
        }
    }

    /// Render a canonical [`HookResponse`] verdict into this vendor's native
    /// output contract as a stdout-ready JSON string. The event defaults to the
    /// one stamped on the response; use [`Self::render_for`] when the answered
    /// event is known independently (a gate may emit a bare verdict).
    #[must_use]
    pub fn render(self, resp: &HookResponse) -> String {
        let event = resp
            .hook_specific_output
            .as_ref()
            .map_or("", |h| h.hook_event_name.as_str());
        self.render_for(resp, event)
    }

    /// Render a verdict scoped to the canonical `event` being answered. Only
    /// Cursor's output contract is event-dependent (its `Stop` differs from its
    /// permission events); Claude Code and Codex render identically regardless.
    #[must_use]
    pub fn render_for(self, resp: &HookResponse, event: &str) -> String {
        match self {
            Self::ClaudeCode => {
                serde_json::to_string(resp).unwrap_or_else(|_| claude_fallback_block())
            }
            Self::Cursor => cursor::render(resp, event),
            Self::Codex => codex::render(resp),
        }
    }

    /// The process exit code this vendor expects to signal a hard block. Claude
    /// Code and Cursor signal via the JSON body (exit 0); Codex blocks with exit 2.
    #[must_use]
    pub const fn block_exit_code(self) -> i32 {
        match self {
            Self::ClaudeCode | Self::Cursor => 0,
            Self::Codex => 2,
        }
    }
}

/// Last-ditch Claude-Code block JSON if the canonical response fails to serialize
/// (mirrors the existing `write_json` fallback so behavior is identical).
fn claude_fallback_block() -> String {
    r#"{"decision":"block","reason":"hook internal error: serialization failed"}"#.to_owned()
}
