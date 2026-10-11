# Agent Harness Integration Guide

Kavach acts as a unified governor across all major AI agent harnesses. One binary manages hooks, rules, and memory for all of them.

---

## 1. Claude Code

Kavach hooks directly into Claude Code's lifecycle events:
- **`PreToolUse`**: Intercepts unconstrained whole-file reads and hazardous shell commands.
- **`PostToolUse`**: Sanitizes command outputs and audits secret leakages.
- **`Stop`**: Blocks session termination until all verification gates (`kavach verify`) pass.

### Configuration (`~/.claude/settings.json`)
```json
{
  "hooks": {
    "PreToolUse": "kavach gates pre-tool",
    "PostToolUse": "kavach gates post-tool",
    "Stop": "kavach gates stop"
  }
}
```

---

## 2. Cursor

Cursor interacts with Kavach via its `.cursor/rules` and prompt submission hooks:
- Kavach injects the live mistake ledger and active roadmap status into every Cursor turn via `beforeSubmitPrompt`.
- Add `.cursor/rules/kavach.mdc` to enforce sovereign CLI proxy usage.

---

## 3. Codex

Codex shares the standard lifecycle channels (`SessionStart`, `UserPromptSubmit`):
- Append hook configurations to `~/.codex/config.toml`:
```toml
[features]
hooks = true

[hooks]
session_start = "kavach session init"
user_prompt = "kavach working-state inject"
```
