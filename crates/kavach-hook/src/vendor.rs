// hub: multi-harness native edge — vendor detection + per-vendor input lowering
// and output rendering. The ONLY place in the tree that knows a non-Claude-Code
// harness exists; the engine/gates/DB stay vendor-blind (decision.multi-harness-native-edges).
//! Native edges for the harnesses kavach runs inside — Claude Code, Cursor,
//! Codex, Antigravity, Pi.
//!
//! Each harness spawns the same kavach binary and pipes JSON over stdin/stdout,
//! but in its OWN dialect. This module is the anti-corruption layer: it detects
//! which harness called (hybrid: explicit override wins, else payload sniff,
//! else env marker for lifecycle events whose payload is CC-shaped),
//! LOWERS that harness's native input into the canonical [`HookInput`] pivot the
//! gates reason over, and RENDERS the canonical [`HookResponse`] verdict back
//! into that harness's native output contract — including its native failure
//! policy (Cursor fails OPEN; Codex/Claude Code fail closed).
//!
//! SOURCES: <https://cursor.com/docs/hooks> · <https://developers.openai.com/codex/hooks>
use kavach_types::{HookInput, HookResponse};
pub mod antigravity;
pub mod codex;
pub mod cursor;
pub mod pi;
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
    /// Google Antigravity (`agy`) CLI — successor to the retired Gemini CLI.
    /// `PascalCase` CC-compatible events; output is `{"decision":"allow"|"deny"}`
    /// (block via the JSON body, NOT an exit code). Config at
    /// `~/.gemini/config/hooks.json`.
    Antigravity,
    /// Pi (`earendil-works/pi`) coding agent — TS extensions via `pi.on(event,cb)`.
    /// A `tool_call` handler returns `{"block":true,"reason":…}` to deny and
    /// NOTHING to allow (block via the JSON body, NOT an exit code). `agent_end` is
    /// Pi's Stop-equivalent. Extension at `~/.pi/agent/extensions/kavach/index.ts`.
    Pi,
}
/// The env var that force-selects a vendor, overriding payload auto-detect.
pub const VENDOR_ENV: &str = "KAVACH_HARNESS";
/// The live upstream source for a vendor's hook-contract schema.
///
/// Lets Kavach reference the CURRENT contract from the internet when a vendor
/// updates its format, instead of trusting a frozen in-binary assumption.
///
/// The kind is type-encoded so callers never mistake a prose page for a fetchable
/// schema: only [`SchemaSource::JsonSchema`] is a machine-readable document a
/// drift-watcher may GET and diff; [`SchemaSource::Prose`] is human reference for
/// a vendor that publishes no JSON Schema endpoint (Codex/Antigravity, Jun 2026).
/// SOURCES: json.schemastore.org · cursor.com/docs/hooks ·
/// developers.openai.com/codex/hooks · antigravity.google/docs/hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaSource {
    /// A machine-readable JSON Schema document — fetchable and diffable. `prose`
    /// is the companion human-readable contract page.
    JsonSchema {
        /// URL of the JSON Schema document (the diff target for drift-watching).
        url: &'static str,
        /// Companion human-readable contract docs.
        prose: &'static str,
    },
    /// A human-readable contract page only; the vendor publishes no JSON Schema
    /// endpoint, so a drift-watcher can surface this URL but cannot auto-diff it.
    Prose {
        /// URL of the contract documentation.
        url: &'static str,
    },
}
impl SchemaSource {
    /// The primary URL to reference for this vendor — the JSON Schema when one
    /// exists, else the prose contract page.
    #[must_use]
    pub const fn url(&self) -> &'static str {
        match *self {
            Self::JsonSchema { url, .. } | Self::Prose { url } => url,
        }
    }
    /// `true` when this source is a machine-readable JSON Schema a drift-watcher
    /// may fetch and diff (vs. prose a human must read).
    #[must_use]
    pub const fn is_machine_readable(&self) -> bool {
        matches!(self, Self::JsonSchema { .. })
    }
}
impl Vendor {
    /// Parse a vendor tag (CLI `--vendor` value or `KAVACH_HARNESS`), case-insensitive.
    /// `None` for an empty/unknown tag so the caller falls through to auto-detect.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag.trim().to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" | "cc" => Some(Self::ClaudeCode),
            "cursor" => Some(Self::Cursor),
            "codex" => Some(Self::Codex),
            "antigravity" | "agy" | "gemini" => Some(Self::Antigravity),
            "pi" => Some(Self::Pi),
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
    /// Auto-detect the vendor from the raw payload's shape, then — only if the
    /// payload is inconclusive — from the process environment.
    ///
    /// Payload signals are preferred because they are per-invocation and cannot be
    /// stale. But several lifecycle events carry NO distinguishing payload field:
    /// Cursor's `workspaceOpen` omits `conversation_id`/`generation_id`/`model`,
    /// and Codex's `SessionStart`/`Stop`/`PreCompact`/`UserPromptSubmit` carry no
    /// `turn_id` — their base shape is byte-for-byte a Claude Code payload. For
    /// those, the harness's exported env markers are the only reliable tell, so we
    /// fall back to them rather than silently defaulting to Claude Code.
    /// SOURCES: <https://cursor.com/docs/hooks> · <https://developers.openai.com/codex/hooks>
    #[must_use]
    pub fn detect(raw_payload: &str) -> Self {
        if let Some(v) = Self::detect_from_payload(raw_payload) {
            return v;
        }
        Self::detect_from_env().unwrap_or_default()
    }
    /// Detect from payload shape alone. `None` when the payload carries no
    /// vendor-distinguishing signal (a bare Claude-Code-shaped object, or any
    /// non-object) — the caller then consults the environment.
    ///
    /// `pub(crate)` so tests can assert the payload-shape contract WITHOUT the
    /// ambient env interference of [`detect`] (which falls back to env markers a
    /// parent harness process may set, e.g. `CURSOR_AGENT` under the Cursor IDE).
    pub(crate) fn detect_from_payload(raw_payload: &str) -> Option<Self> {
        let v = serde_json::from_str::<serde_json::Value>(raw_payload).ok()?;
        let has = |k: &str| v.get(k).is_some_and(|x| !x.is_null());
        let event = v.get("hook_event_name").and_then(|e| e.as_str());
        // Cursor: any of its unique top-level fields, OR a camelCase event name
        // from its vocabulary — the latter is the ONLY signal on `workspaceOpen`,
        // which omits every id field but still names a Cursor-only event.
        if has("conversation_id")
            || has("workspace_roots")
            || has("generation_id")
            || has("cursor_version")
            || event.is_some_and(is_cursor_event)
        {
            return Some(Self::Cursor);
        }
        // Codex: `turn_id` is its turn-scope extension over the CC contract. (Its
        // non-turn events are payload-indistinguishable from CC — caught by env.)
        if has("turn_id") {
            return Some(Self::Codex);
        }
        None
    }
    /// Detect from harness-exported env markers — the cross-event fallback for
    /// lifecycle events whose payload is Claude-Code-shaped. `None` when no marker
    /// is set, so the caller defaults to Claude Code.
    ///
    /// - Cursor sets `CURSOR_TRACE_ID` / `CURSOR_AGENT` during agent sessions.
    /// - Codex exports the BARE `PLUGIN_ROOT` (Claude Code only ever sets the
    ///   `CLAUDE_PLUGIN_ROOT` alias) and runs under `CODEX_HOME`.
    fn detect_from_env() -> Option<Self> {
        let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
        if set("CURSOR_TRACE_ID") || set("CURSOR_AGENT") {
            return Some(Self::Cursor);
        }
        // Bare PLUGIN_ROOT without the CLAUDE_ alias is Codex's tell; CODEX_HOME
        // confirms a Codex process even when no plugin is loaded.
        if (set("PLUGIN_ROOT") && !set("CLAUDE_PLUGIN_ROOT")) || set("CODEX_HOME") {
            return Some(Self::Codex);
        }
        None
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
            Self::Antigravity => antigravity::lower(raw_payload),
            Self::Pi => pi::lower(raw_payload),
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
            Self::Antigravity => antigravity::render(resp),
            Self::Pi => pi::render(resp),
        }
    }
    /// The process exit code this vendor expects to signal a hard block. Claude
    /// Code, Cursor, Antigravity, and Pi signal via the JSON body (exit 0 — Pi
    /// blocks with `{"block":true}`, Antigravity with `{"decision":"deny"}`, not
    /// an exit code); Codex blocks with exit 2.
    #[must_use]
    pub const fn block_exit_code(self) -> i32 {
        match self {
            Self::ClaudeCode | Self::Cursor | Self::Antigravity | Self::Pi => 0,
            Self::Codex => 2,
        }
    }
    /// The live upstream hook-contract schema source for this vendor, so Kavach
    /// can reference the CURRENT contract from the internet rather than a frozen
    /// in-binary assumption. Claude Code and Cursor publish a machine-readable
    /// JSON Schema (diffable by a drift-watcher); Codex and Antigravity publish
    /// only a prose contract page as of Jun 2026.
    ///
    /// URLs are 2-source corroborated. SOURCES: schemastore.org · cursor.com ·
    /// developers.openai.com/codex · antigravity.google.
    #[must_use]
    pub const fn schema_url(self) -> SchemaSource {
        match self {
            Self::ClaudeCode => SchemaSource::JsonSchema {
                url: "https://json.schemastore.org/claude-code-settings.json",
                prose: "https://code.claude.com/docs/en/hooks",
            },
            Self::Cursor => SchemaSource::JsonSchema {
                url: "https://unpkg.com/cursor-hooks/schema/hooks.schema.json",
                prose: "https://cursor.com/docs/hooks",
            },
            Self::Codex => SchemaSource::Prose {
                url: "https://developers.openai.com/codex/hooks",
            },
            Self::Antigravity => SchemaSource::Prose {
                url: "https://antigravity.google/docs/hooks",
            },
            Self::Pi => SchemaSource::Prose {
                url: "https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md",
            },
        }
    }
    /// Every vendor in detection order — the canonical roster a `--all` schema
    /// listing iterates. Kept in sync with [`Self`]'s variants by exhaustiveness:
    /// adding a variant forces this array (and `schema_url`) to be updated.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::ClaudeCode,
            Self::Cursor,
            Self::Codex,
            Self::Antigravity,
            Self::Pi,
        ]
    }
    /// Stable lowercase name for reports and `--vendor` round-trips. Defined here
    /// (not duplicated in callers) so the `#[non_exhaustive]` enum stays the SOLE
    /// match site — adding a variant forces an update in exactly one place.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
            Self::Pi => "pi",
            Self::Antigravity => "antigravity",
        }
    }
}
/// True if `event` is a Cursor camelCase hook event. Cursor is the only harness
/// whose event names are camelCase (`beforeShellExecution`, `workspaceOpen`…);
/// Claude Code and Codex both use `PascalCase` (`PreToolUse`, `SessionStart`), so a
/// match here is an unambiguous Cursor tell — and the SOLE signal on lifecycle
/// events like `workspaceOpen` that omit every id field.
/// SOURCE: <https://cursor.com/docs/hooks>
fn is_cursor_event(event: &str) -> bool {
    matches!(
        event,
        "sessionStart"
            | "sessionEnd"
            | "preToolUse"
            | "postToolUse"
            | "postToolUseFailure"
            | "subagentStart"
            | "subagentStop"
            | "beforeShellExecution"
            | "afterShellExecution"
            | "beforeMCPExecution"
            | "afterMCPExecution"
            | "beforeReadFile"
            | "afterFileEdit"
            | "beforeSubmitPrompt"
            | "preCompact"
            | "stop"
            | "afterAgentResponse"
            | "afterAgentThought"
            | "beforeTabFileRead"
            | "afterTabFileEdit"
            | "workspaceOpen"
    )
}
/// Last-ditch Claude-Code block JSON if the canonical response fails to serialize
/// (mirrors the existing `write_json` fallback so behavior is identical).
fn claude_fallback_block() -> String {
    r#"{"decision":"block","reason":"hook internal error: serialization failed"}"#.to_owned()
}
