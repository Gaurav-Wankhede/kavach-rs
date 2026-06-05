<div align="center">
  <img src="docs/branding/kavach-logo-512.png" alt="Kavach" width="160" height="160">
  <h1>Kavach</h1>
  <p><strong>A self-improving development harness for Claude Code that enforces engineering discipline through lifecycle gates, persistent memory, and a knowledge graph.</strong></p>
</div>

---

Kavach (Sanskrit: कवच, "armor") wraps every Claude Code session in a verification layer. It catches permission-seeking, enforces research-before-code, blocks destructive operations, remembers decisions across sessions, and learns from mistakes — all through Claude Code's native hook system.

---

## Why Kavach?

Claude Code is powerful but stateless between sessions and easily slips into anti-patterns: asking permission instead of acting, skipping research, repeating past mistakes, leaving work half-done. Kavach fixes this with a **Rust hook engine** that runs at every lifecycle event.

| Without Kavach | With Kavach |
|----------------|-------------|
| Asks "should I proceed?" mid-task | L4 autonomy — acts, reports after |
| Re-researches solved problems | Persistent decision memory |
| Repeats past mistakes | Mistake ledger with embedding clustering |
| Loses context on compact | State checkpointed to the DB, not the chat |
| Ships half-finished work | Stop-gate blocks until a 3-witness verify |
| Destructive `rm -rf` slips through | Pre-tool guards block or ask |

---

## How It Works

```
┌──────────────────────────────────────────────────────────────┐
│  Claude Code Session                                          │
│                                                              │
│  User prompt  → [UserPromptSubmit gate] → intent analysis    │
│       ↓                                                      │
│  Tool call    → [PreToolUse gate]       → block / ask / allow│
│       ↓                                                      │
│  Tool result  → [PostToolUse gate]      → research + memory  │
│       ↓                                                      │
│  Stop         → [Stop gate]             → 3-witness or block │
└──────────────────────────────────────────────────────────────┘
         ↓ every gate routes through ↓
┌──────────────────────────────────────────────────────────────┐
│  kavach-rpc daemon (SurrealDB-backed)                         │
│  decisions · research · patterns · roadmap · mistakes · KG    │
└──────────────────────────────────────────────────────────────┘
```

Each hook event invokes the `kavach` binary, which routes through a persistent RPC daemon backed by SurrealDB. Gates are pure Rust functions that inspect the event and return an allow / block / ask decision — keeping a single DB writer and shared state out of the conversation window.

---

## Quick Start

### Install the desktop app (GUI + CLI in one file)

Each installer below bundles **both** the Kavach desktop GUI and the `kavach` CLI in a single setup file. The `latest/download/` links always resolve to the most recent release:

| Platform | Single setup file | Installs |
|----------|-------------------|----------|
| macOS (Apple Silicon) | [`Kavach-macos-arm64.dmg`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/Kavach-macos-arm64.dmg) | `KavachApp.app` → `/Applications` (CLI embedded at `Contents/MacOS/kavach`) |
| Linux (x86_64) | [`Kavach-linux-amd64.deb`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/Kavach-linux-amd64.deb) | GUI + `kavach` CLI → `/usr/bin` (`sudo dpkg -i …`) |
| Windows (x86_64) | [`Kavach-windows-amd64.msi`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/Kavach-windows-amd64.msi) | GUI + `kavach.exe` → `C:\Program Files\KavachApp` |

These installers are **ad-hoc signed** (no paid Apple/Microsoft certificate): on macOS, right-click → Open the first time; on Windows, "More info → Run anyway" past SmartScreen.

> **Versioning — `YY.MM.PATCH`** (CalVer + SemVer blend, JetBrains/Ubuntu style). `YY.MM` is the release calendar month; `PATCH` is the iteration within it. Examples: `26.5.0` (first May-2026 release), `26.5.1`, `26.6.0`. A release is cut by pushing a matching git tag — the tag *is* the version baked into every installer.

### Download the CLI only (prebuilt binary)

Just want the `kavach` CLI? Grab the archive for your platform from the [latest release](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest):

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | [`kavach-linux-amd64.tar.gz`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/kavach-linux-amd64.tar.gz) |
| Linux | aarch64 | [`kavach-linux-arm64.tar.gz`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/kavach-linux-arm64.tar.gz) |
| macOS | x86_64 (Intel) | [`kavach-darwin-amd64.tar.gz`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/kavach-darwin-amd64.tar.gz) |
| macOS | aarch64 (Apple Silicon) | [`kavach-darwin-arm64.tar.gz`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/kavach-darwin-arm64.tar.gz) |
| Windows | x86_64 | [`kavach-windows-amd64.zip`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/kavach-windows-amd64.zip) |

Verify the download against [`SHA256SUMS.txt`](https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/SHA256SUMS.txt) from the same release.

**Linux / macOS:**

```bash
# Pick the asset for your platform (Apple Silicon example shown)
curl -fsSL -o kavach.tar.gz \
  https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/kavach-darwin-arm64.tar.gz
tar -xzf kavach.tar.gz
mkdir -p ~/.local/bin && mv kavach ~/.local/bin/kavach
kavach --version
```

**Windows (PowerShell):**

```powershell
Invoke-WebRequest -Uri `
  https://github.com/Wankhede-Brothers/kavach-rs/releases/latest/download/kavach-windows-amd64.zip `
  -OutFile kavach.zip
Expand-Archive kavach.zip -DestinationPath "$env:USERPROFILE\.local\bin" -Force
& "$env:USERPROFILE\.local\bin\kavach.exe" --version
```

### Build from source

Requires **Rust 1.96+** (edition 2024) and **Claude Code** v2.0+.

```bash
# Clone
git clone https://github.com/Wankhede-Brothers/kavach-rs
cd kavach-rs

# Build the release binary (the package is kavach-cli; the binary is `kavach`)
cargo build --release

# Symlink into PATH (Linux/macOS)
ln -sf "$(pwd)/target/release/kavach" ~/.local/bin/kavach

# Verify
kavach --version
```

### Wire Into Claude Code

Map each Claude Code lifecycle event to a Kavach gate in `~/.claude/settings.json`. The gate names are `intent`, `pre-write`, `post-write`, `pre-tool`, `post-tool`, `session-start`, and `stop` — every gate runs the same way (`kavach gates <name> --hook`, reading the hook JSON from stdin).

```json
{
  "hooks": {
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "kavach gates intent --hook" }] }],
    "PreToolUse": [
      { "matcher": "Write|Edit|NotebookEdit", "hooks": [{ "type": "command", "command": "kavach gates pre-write --hook" }] },
      { "matcher": "*",                        "hooks": [{ "type": "command", "command": "kavach gates pre-tool --hook" }] }
    ],
    "PostToolUse": [
      { "matcher": "Write|Edit|NotebookEdit", "hooks": [{ "type": "command", "command": "kavach gates post-write --hook" }] },
      { "matcher": "*",                        "hooks": [{ "type": "command", "command": "kavach gates post-tool --hook" }] }
    ],
    "SessionStart": [{ "hooks": [{ "type": "command", "command": "kavach gates session-start --hook" }] }],
    "Notification":   [{ "hooks": [{ "type": "command", "command": "kavach gates notification --hook" }] }],
    "MessageDisplay": [{ "hooks": [{ "type": "command", "command": "kavach gates message-display --hook" }] }],
    "Stop":         [{ "hooks": [{ "type": "command", "command": "kavach gates stop --hook" }] }]
  }
}
```

Kavach tracks Claude Code's hook surface as it evolves. Recent adoptions: the
`message-display` gate (CC 2.1.152 `MessageDisplay`), `reloadSkills` + `sessionTitle`
on `session-start` (CC 2.1.152), the `effort.level` / `$CLAUDE_EFFORT` hook input that
lets gates modulate strictness by effort tier (CC 2.1.133), `terminalSequence` bells on
attention-needing notifications (CC 2.1.141), and `ultracode` intent recognition (CC 2.1.160).

The `pre-write`/`post-write` gates carry the hard enforcement (skills, research, anti-pattern scan) on file mutations; `pre-tool`/`post-tool` cover every other tool (Bash blocklist, context injection, research tracking).

---

## Core Concepts

### Gates

Gates are the enforcement primitive. Each gate is a Rust function mapped to a Claude Code lifecycle event. A gate can **block** (deny the action), **ask** (require confirmation), or **allow** (pass through, optionally injecting context). Detectors live in `kavach-patterns`; the dispatch and severity wiring live in `kavach-engine`.

### Memory (kavach-db)

A SurrealDB store holds durable state across sessions, scoped per project:

- **decisions** — architectural choices, never re-litigated
- **research** — findings from web searches, cached
- **patterns** — gate false-positive fixes, learned over time
- **roadmap** — kanban-style task tracking (the kanban is a status lens over this)
- **app_spec** — six-file project context

All access is RPC-routed through the daemon — no crate opens the database directly, preserving the single-writer invariant.

### Knowledge Graph

A three-tier graph: global **concepts** (L0) link to project **entities** (L1), and **mistakes** cluster into anti-patterns (L3) using BGE-small embeddings with cosine similarity.

### Rules Engine

A dedicated crate family (`kavach-rule-*`) parses, stores, and evaluates rule definitions — the declarative layer that drives skill keyword routing and gate advisories.

### Autonomous Harness Loop

Kavach picks the right *agentic workflow shape* for a task and drives it autonomously — no manual orchestration. A task is classified into one of six dynamic-workflow patterns (after Anthropic's [Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents)):

| Pattern | Shape | When |
|---------|-------|------|
| `classify-act` | route → handle | triage / dispatch-by-type |
| `fan-out-synthesize` | parallel → merge | audits, broad sweeps |
| `worker-critic` | produce → adversarially verify | reviews, hardening |
| `generate-filter` | many candidates → keep best | brainstorming, design search |
| `pairwise-tournament` | compare → rank → pick winner | "best of N" |
| `loop-until-done` | iterate to a goal oracle | open-ended build/fix (default) |

The loop is **DB-driven, end to end**:

1. **Classify (intent gate)** — `UserPromptSubmit` keyword-routes the prompt to a pattern and persists it on the next-open roadmap card via `db.set_harness`.
2. **Schema (kavach-db)** — each card carries `harness` + `workflow_path` columns.
3. **Dispatch (stop gate)** — when a card with a harness is claimed, the Stop gate reads the link (`db.get_harness`) plus the oracle's last verdict (`db.latest_goal_attempt`) and emits `[AUTO_CONTINUE] run Workflow <path>` — commanding the AI to run the compiled `workflow.js` rather than hand-execute the card.
4. **Compile** — `kavach goal compile` turns a `loop.yaml` (tagged-enum harness spec) into a Claude Code Workflow.

The card's harness link in the DB is the single source of truth, so the choice survives context compaction and session restarts.

---

## Architecture

The workspace is 20 crates. The load-bearing ones:

```
kavach-rs/
├── crates/
│   ├── kavach-cli/          # binary entry point (`kavach`) + CLI commands
│   ├── kavach-engine/       # gate dispatch, severity wiring, DAG team scheduler
│   ├── kavach-chain/        # verification chain runner
│   ├── kavach-patterns/     # pattern detectors (the guards gates fire)
│   ├── kavach-rpc/          # JSON-RPC daemon + client
│   ├── kavach-surreal/      # SurrealDB persistence + knowledge graph
│   ├── kavach-session/      # cross-turn session state + mistake ledger
│   ├── kavach-types/        # shared types (HookInput, MemoryStatus, Priority)
│   ├── kavach-config/       # configuration loading
│   ├── kavach-hook/         # Claude Code hook I/O + lifecycle plumbing
│   ├── kavach-advisor/      # advisory client + types
│   ├── kavach-dtree/        # decision-tree primitives (intent classification)
│   ├── kavach-rag-core/     # retrieval scan / score / tree walk
│   ├── kavach-toon/         # token-efficient serialization
│   ├── kavach-app/          # desktop/web UI (knowledge-graph viewer)
│   └── kavach-rule-*/       # rule ast · parser · engine · generator · storage
└── (skills are loaded from ~/.claude/skills at runtime)
```

---

## Development

Kavach follows a strict-lint, evidence-gated workflow: edition 2024, `unsafe` forbidden workspace-wide, `dead_code` denied, and lib crates use `thiserror` while the app uses `anyhow`.

```bash
# Build all crates
cargo check --workspace

# Run tests (nextest — parallel, per-test process isolation)
cargo nextest run --workspace

# Lint (strict — correctness lints are deny-by-default)
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all
```

The "done" bar is a **3-witness verify**: an `rg` artifact (the change exists at file:line), `git diff --stat` (the diff landed), and `cargo check --workspace` exit 0 (it compiles). The Stop gate blocks until those hold.

---

## Changelog

Release history lives in [CHANGELOG.md](CHANGELOG.md), newest version first.

---

## License

[MIT](LICENSE) © 2026 Wankhede Brothers
