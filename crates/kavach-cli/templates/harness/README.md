# Kavach multi-harness install configs

One kavach binary, one DB, three native front-doors. Pick your IDE, paste the
matching config, and kavach runs natively — detecting the dialect, lowering each
tool's payload into its canonical pivot, and rendering each tool's native output.

| IDE | File | Install path | --vendor |
|-----|------|-------------|----------|
| Claude Code | `claude.settings.json` | `~/.claude/settings.json` (merge `hooks`) | none (auto) |
| Cursor | `cursor.hooks.json` | `~/.cursor/hooks.json` or `<project>/.cursor/hooks.json` | `cursor` |
| Codex | `codex.config.toml` | append to `~/.codex/config.toml`; set `[features] hooks = true` | `codex` |

`--vendor` is optional everywhere — omit it and kavach auto-detects from the
payload (`conversation_id`→Cursor, `turn_id`→Codex, else Claude Code). The flag
just forces the dialect for certainty. `$KAVACH_HARNESS` is a third override.

All three share the SAME kavach DB: a Cursor session, a Codex session, and a
Claude Code session are each first-class rows keyed by their native session id
(Cursor's `conversation_id` is normalized to the pivot `session_id` at the door).
Run Cursor for one task and Codex for another — they read/write one memory bank.

Failure policy is native per vendor: Cursor fails OPEN (a hook error never wedges
the IDE), Codex and Claude Code fail CLOSED (exit 2 / block).

SOURCES: <https://cursor.com/docs/hooks> · <https://developers.openai.com/codex/hooks>
· <https://code.claude.com/docs/en/hooks>
