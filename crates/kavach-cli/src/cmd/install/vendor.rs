//! Install targets: which native tool, where its OFFICIAL hook config lives, and
//! the ready-to-write template body. Each tool loads Kavach via its OWN official
//! mechanism (Cursor hooks.json, Codex config.toml [[hooks]], CC settings.json,
//! Antigravity hooks.json, Pi extension) — a native install, NOT a shim.
//! SOURCE: decision.kavach-universal-subscription-substrate.

/// A native tool Kavach can install itself into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    ClaudeCode,
    Cursor,
    Codex,
    /// Google Antigravity (`agy`) — successor to the retired Gemini CLI. Named
    /// Antigravity throughout (user directive); `gemini` is only a `from_tag` alias.
    Antigravity,
    Pi,
}

/// The embedded template body per tool, baked into the binary at compile time so
/// `kavach install` works without the source tree present.
const CLAUDE: &str = include_str!("../../../templates/harness/claude.settings.json");
const CURSOR: &str = include_str!("../../../templates/harness/cursor.hooks.json");
const CODEX: &str = include_str!("../../../templates/harness/codex.config.toml");
const ANTIGRAVITY: &str = include_str!("../../../templates/harness/antigravity.hooks.json");
const PI: &str = include_str!("../../../templates/harness/pi.index.ts");

/// Engineering-directives templates (shipped for cc/cursor/codex only).
const CLAUDE_MD: &str = include_str!("../../../templates/harness/CLAUDE.md");
const MDC: &str = include_str!("../../../templates/harness/kavach.mdc");
const AGENTS: &str = include_str!("../../../templates/harness/AGENTS.md");

impl Target {
    /// Parse a `--vendor` tag, case-insensitive. `None` for an unknown tag.
    pub(super) fn from_tag(tag: &str) -> Option<Self> {
        match tag.trim().to_ascii_lowercase().as_str() {
            "cc" | "claude" | "claude-code" | "claudecode" => Some(Self::ClaudeCode),
            "cursor" => Some(Self::Cursor),
            "codex" => Some(Self::Codex),
            "antigravity" | "agy" | "gemini" => Some(Self::Antigravity),
            "pi" => Some(Self::Pi),
            _ => None,
        }
    }

    /// Every target installed by `--vendor all`. All five ship a template now
    /// (Pi's is a TypeScript extension shim), so `all` installs the full set.
    pub(super) const fn all() -> &'static [Self] {
        &[
            Self::ClaudeCode,
            Self::Cursor,
            Self::Codex,
            Self::Antigravity,
            Self::Pi,
        ]
    }

    /// Stable lowercase name for messages and `--vendor` round-trips.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
            Self::Antigravity => "antigravity",
            Self::Pi => "pi",
        }
    }

    /// Path under `$HOME` where this tool reads its OFFICIAL hook config.
    /// SOURCES: code.claude.com/docs/settings · cursor.com/docs/hooks ·
    /// developers.openai.com/codex · ai.google.dev · github.com/earendil-works/pi.
    pub(super) const fn rel_config_path(self) -> &'static str {
        match self {
            Self::ClaudeCode => ".claude/settings.json",
            Self::Cursor => ".cursor/hooks.json",
            Self::Codex => ".codex/config.toml",
            // Antigravity (agy) — shared hooks path per antigravity-cli CHANGELOG v1.0.8.
            Self::Antigravity => ".gemini/config/hooks.json",
            Self::Pi => ".pi/agent/extensions/kavach/index.ts",
        }
    }

    /// The embedded template body for this target. Every shipped target has one;
    /// a future not-yet-built target would reintroduce an `Option` here (and the
    /// fail-closed unshipped-error path in `install_one`) with its first `None`.
    pub(super) const fn template(self) -> &'static str {
        match self {
            Self::ClaudeCode => CLAUDE,
            Self::Cursor => CURSOR,
            Self::Codex => CODEX,
            // Gemini == Antigravity (agy): the legacy gemini CLI retired 2026-06-18.
            Self::Antigravity => ANTIGRAVITY,
            // Pi: TypeScript extension shim (not JSON/TOML config) at the auto-
            // discovery path; shells to the kavach binary with --vendor pi.
            Self::Pi => PI,
        }
    }

    /// True when this tool's config is TOML (append-merge) rather than JSON.
    pub(super) const fn is_toml(self) -> bool {
        matches!(self, Self::Codex)
    }

    /// Path under `$HOME` for this tool's directives doc; `None` if it has none.
    pub(super) const fn rel_directives_path(self) -> Option<&'static str> {
        match self {
            Self::ClaudeCode => Some(".claude/CLAUDE.md"),
            Self::Cursor => Some(".cursor/rules/kavach.mdc"),
            Self::Codex => Some(".codex/AGENTS.md"),
            Self::Antigravity | Self::Pi => None,
        }
    }

    /// The embedded directives template body, paired with `rel_directives_path`.
    pub(super) const fn directives_template(self) -> Option<&'static str> {
        match self {
            Self::ClaudeCode => Some(CLAUDE_MD),
            Self::Cursor => Some(MDC),
            Self::Codex => Some(AGENTS),
            Self::Antigravity | Self::Pi => None,
        }
    }
}
