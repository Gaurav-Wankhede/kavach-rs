<div align="center">

<img src="docs/branding/kavach-logo-512.png" alt="Kavach" width="150" height="150">

<h1>Kavach</h1>

<p>
  <strong>A self-improving development harness for AI coding agents.</strong><br>
  Lifecycle gates that enforce engineering discipline · persistent cross-session memory ·<br>
  a knowledge graph that learns from every mistake — all in one Rust binary.
</p>

<p>
  <a href="https://github.com/Wankhede-Brothers/kavach-rs/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/Wankhede-Brothers/kavach-rs?style=for-the-badge&color=2b6cb0&label=release"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-22863a?style=for-the-badge"></a>
  <img alt="Rust 1.96+" src="https://img.shields.io/badge/Rust-1.96%2B-dea584?style=for-the-badge&logo=rust&logoColor=white">
  <img alt="Edition 2024" src="https://img.shields.io/badge/edition-2024-555?style=for-the-badge">
</p>

<p>
  <strong>Claude Code</strong> · <strong>Codex</strong> · <strong>Cursor</strong> — one binary, one shared memory bank, three harnesses.
</p>

<sub>कवच &nbsp;·&nbsp; Sanskrit for <em>"armor"</em> — Kavach wraps every agent session in a verification layer.</sub>

</div>

<hr>

<div align="center">
<table>
<tr>
<td align="center" width="33%">
<h3>🛡️ Enforce</h3>
Lifecycle gates catch permission-seeking, skipped research, destructive ops, and half-done work — and <em>block</em> them.
</td>
<td align="center" width="33%">
<h3>🧠 Remember</h3>
Decisions, research, and mistakes persist in SurrealDB — surviving context compaction and session restarts.
</td>
<td align="center" width="33%">
<h3>♻️ Improve</h3>
The harness auto-<strong>feeds</strong> its memory and auto-<strong>recalls</strong> it on every prompt. Gaps become tracked work.
</td>
</tr>
</table>
</div>

<div align="center">
<sub>🌐 <strong>Internet-first, enforced.</strong> Every prompt is pinned to your <em>installed</em> dependency versions (from the lockfile) and handed the exact registry URL to fetch the <em>latest</em> — so the agent confirms upstream instead of guessing from stale training weights.</sub>
</div>

<hr>

## Why Kavach?

AI coding agents are powerful but **stateless between sessions** and easily slip into anti-patterns: asking permission instead of acting, skipping research, repeating past mistakes, leaving work half-done. Kavach fixes this with a **Rust hook engine** that runs at every lifecycle event.

<div align="center">
<table>
<tr><th>Without Kavach</th><th>With Kavach</th></tr>
<tr><td>Asks <em>"should I proceed?"</em> mid-task</td><td><strong>L4 autonomy</strong> — acts, reports after</td></tr>
<tr><td>Re-researches solved problems</td><td>Persistent decision memory</td></tr>
<tr><td>Repeats past mistakes</td><td>Mistake ledger with embedding clustering</td></tr>
<tr><td>Loses context on compact</td><td>State checkpointed to the DB, not the chat</td></tr>
<tr><td>Ships half-finished work</td><td>Stop-gate blocks until a <strong>3-witness verify</strong></td></tr>
<tr><td>Destructive <code>rm -rf</code> slips through</td><td>Pre-tool guards block or ask</td></tr>
<tr><td>Writes memory it never reads back</td><td><strong>Brain-OS auto-recall</strong> injects relevant memory into every prompt</td></tr>
<tr><td>Claims "latest version" from stale weights</td><td><strong>Internet-first</strong> — installed pinned from the lockfile, latest fetched from the registry</td></tr>
<tr><td>Fabricates a CLI subcommand/flag that doesn't exist</td><td><strong>No-fabrication recovery</strong> — a failed <code>kavach</code> call routes to <code>kavach commands --tree</code>/<code>--help</code>; a stale binary triggers <code>just install</code></td></tr>
<tr><td>Context rots as verbose injections re-enter every turn</td><td><strong>Injection compaction</strong> — every gate injection is compressed at the emit chokepoint (grammar dropped, code/URLs/tokens preserved byte-for-byte)</td></tr>
<tr><td>Reads a gate denial as "BLOCKED", surrenders, ships nothing</td><td><strong>Action-imperative verdicts</strong> — every denial reads <code>[KEYWORD] what → do-this → retry</code> plus a <code>[NEXT_ACTION]</code> trailer; a verdict is a redirect, never a dead end</td></tr>
</table>
</div>

<hr>

## How It Works — System Architecture

Every harness lifecycle event invokes the `kavach` binary as a thin client. Gates are pure Rust functions that inspect the event and return **allow / block / ask**. All durable state lives in a standalone SurrealDB **server** (not a daemon) — every Kavach process is a thin WebSocket client of it, and the server is the single writer.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│   AI HARNESS  (Claude Code · Codex · Cursor — identical hook set)              │
│   settings.json hooks → ~/.local/bin/kavach gates <event> --hook              │
└───────────────┬──────────────────────────────────────────────────────────────┘
                │ lifecycle events
    ┌───────────┼────────────────────────┬──────────────────────────┐
 UserPromptSubmit                   PostToolUse                    Stop
    │                                    │                            │
┌───▼────────────────┐     ┌─────────────▼──────────┐   ┌─────────────▼──────────┐
│ intent GATE        │     │ post-tool GATE         │   │ stop GATE              │
│ ② recall_block() ◄─┼──┐  │ harvest_concepts()     │   │ extract RCA/patterns   │
│ ③ inject [RECALL]  │  │  └──────────┬─────────────┘   └──────────┬─────────────┘
└───┬────────────────┘  │             │ concept.add                │ pattern.add
    │ [RECALL] context   │ brain.think │ (WRITE)                    │ (WRITE)
    ▼ to agent           │ (READ)      ▼                            ▼
                         │  ┌──────────────────────────────────────────────────┐
                         │  │   kavach-rpc  ── IN-PROCESS dispatch ──           │
                         │  │   (NO daemon, NO Unix socket — client.rs)        │
                         │  │   holds ONE long-lived DB handle:                │
                         └─►│   open_default_held()                            │
                            │   verbs: brain.think · concept.* · …             │
                            └──────────────────────┬───────────────────────────┘
                                                   │ ws://127.0.0.1:7710
                                                   │ (thin ws CLIENT; root signin)
                            ┌──────────────────────▼───────────────────────────┐
                            │   surreal start  SERVER   ◄── the real "server"   │
                            │   official SurrealDB binary                       │
                            │   launchd  ai.shared.kavach-surreal  (KeepAlive)  │
                            │   ★ SERIALIZES ALL WRITERS — no file LOCK to hold │
                            └──────────────────────┬───────────────────────────┘
                                                   │ owns on-disk store
                            ┌──────────────────────▼───────────────────────────┐
                            │  kv-rocksdb (BM25 FULLTEXT + concept graph)       │
                            │  per-OS data dir / SharedAI / kavach.surreal      │
                            │  (macOS · Linux · Windows — resolved at runtime)  │
                            └───────────────────────────────────────────────────┘

  every kavach process (gate · CLI · web) = a thin ws CLIENT of the surreal SERVER.
  the SERVER is the single writer. there is no kavach-owned daemon anywhere.
```

> **Two layers, named correctly.** `kavach-rpc` is an **in-process** dispatch (not a daemon, not a socket) that holds one DB handle via `open_default_held()`. The **`surreal start` server** (the official SurrealDB binary, supervised by launchd `ai.shared.kavach-surreal` on `ws://127.0.0.1:7710`) is the single writer — a real server, not a daemon.

<hr>

## 🧠 Brain-OS — the closed self-improving loop

Brain-OS is Kavach's memory + retrieval layer. What makes it different from a passive notes store is that the harness manages it **autonomously in both directions** — the agent never types a query by hand.

<div align="center">
<table>
<tr><th>Phase</th><th>Trigger</th><th>Gate</th><th>Direction</th><th>What happens</th></tr>
<tr><td><strong>WRITE</strong></td><td>WebSearch returns</td><td><code>post-tool</code></td><td>brain&nbsp;←</td><td><code>harvest_concepts()</code> → <code>concept.add</code> → corpus grows on its own</td></tr>
<tr><td><strong>WRITE</strong></td><td>verify passes</td><td><code>stop</code></td><td>brain&nbsp;←</td><td><code>[RCA]</code>/<code>[DESIGN]</code>/patterns extracted into rows</td></tr>
<tr><td><strong>READ</strong></td><td><em>every</em> prompt</td><td><code>intent</code></td><td>brain&nbsp;→</td><td><code>recall_block()</code> → <code>brain.think</code> → RRF → <code>[RECALL]</code> injected into the agent's context</td></tr>
</table>
</div>

**Retrieval is vectorless and explainable.** `kavach think` runs **BM25 full-text** across 5 typed memory tables (`decision · roadmap · research · pattern · app_spec`) plus a **concept-graph** FTS source, then fuses the rank lists with **Reciprocal Rank Fusion** (`k=60`). No embeddings — every hit is a **citable row key** you can `kavach db get`. When a query finds too little, the gap-filer writes a `research.gap.*` card, so the system's own ignorance becomes tracked work.

```bash
kavach think --project myapp "how did we handle auth token refresh?"
# → {"hits":[{"id":"decision.auth.token-refresh","score":0.03},…],"gap_filed":false}
```

<hr>

## ✅ Prerequisites

Install these **before** running Kavach. The harness fails closed — if its memory store is unreachable, gates deny by default — so the SurrealDB server is not optional.

<div align="center">
<table>
<tr><th>Requirement</th><th>Why</th><th>Install</th></tr>
<tr>
<td><strong>SurrealDB</strong> ≥ 3.1<br><sub>(the memory <em>server</em>)</sub></td>
<td>Kavach's durable store. A standalone <code>surreal start</code> server owns the DB and serializes all writers; every <code>kavach</code> process is a thin WebSocket client of it (default <code>ws://127.0.0.1:7710</code>).</td>
<td><strong>macOS / Linux:</strong> <code>curl -sSf https://install.surrealdb.com | sh</code><br><strong>macOS (brew):</strong> <code>brew install surrealdb/tap/surreal</code><br><strong>Windows:</strong> <code>iwr https://windows.surrealdb.com -useb | iex</code><br><sub>or grab a binary from <a href="https://surrealdb.com/install">surrealdb.com/install</a></sub></td>
</tr>
<tr>
<td><strong>An AI harness</strong></td>
<td>Kavach wraps a coding agent. Use any one: <strong>Claude Code</strong> v2.0+, <strong>Codex</strong>, or <strong>Cursor</strong>. One binary serves all three.</td>
<td>See each tool's install docs</td>
</tr>
<tr>
<td><strong>Rust</strong> 1.96+<br><sub>(only to build from source)</sub></td>
<td>Edition 2024 toolchain. Skip if you download a prebuilt <code>kavach</code> binary.</td>
<td><code>rustup toolchain install 1.96</code></td>
</tr>
<tr>
<td><strong>Rust CLI toolbelt</strong><br><sub>(rg · fd · bat · sd · …)</sub></td>
<td>The <code>pre-tool</code> gate steers shell commands to fast Rust equivalents. Provisioned in one command (below) — no manual per-tool install.</td>
<td><code>kavach toolbelt install --yes</code></td>
</tr>
</table>
</div>

> **Start the memory server once**, then keep it running as a background service (`launchd` on macOS, `systemd` on Linux, a Scheduled Task / NSSM service on Windows). Point `rocksdb://` at any writable directory — `<DATA_DIR>` below is a placeholder:
> ```bash
> surreal start --user root --pass root --bind 127.0.0.1:7710 rocksdb://<DATA_DIR>/kavach.surreal
> ```
> Kavach already knows a sensible per-OS default location (macOS: `~/Library/Application Support/SharedAI`, Linux: `$XDG_DATA_HOME/shared-ai`, Windows: `%LOCALAPPDATA%\SharedAI`) and resolves it automatically — you only set a path here if you run `surreal start` yourself. Override the endpoint/creds with the `KAVACH_SURREAL_ENDPOINT` / `KAVACH_SURREAL_USER` / `KAVACH_SURREAL_PASS` environment variables if you bind elsewhere.

<hr>

## 🚀 Quick Start

<details open>
<summary><strong>One command — install from source (recommended)</strong></summary>

<br>

Kavach installs by cloning + building from source, then deleting the clone — no release archives to download. The installer detects your OS + arch, bootstraps the prerequisites, builds the binary into `~/.local/bin`, provisions the Rust CLI **toolbelt** the gates enforce, and cleans up after itself.

**Linux / macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/Wankhede-Brothers/kavach-rs/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/Wankhede-Brothers/kavach-rs/main/install.ps1 | iex
```

What the installer does, in order:

1. **Bootstraps prerequisites** — `git` (must be present), the **Rust toolchain** via `rustup` (the pinned channel from `rust-toolchain.toml` installs itself on build), and **SurrealDB 3.1.4** (hard-pinned on every platform).
2. **Clones** the repo shallow into a temp dir and runs `cargo build --release`.
3. **Installs** the `kavach` binary to `~/.local/bin` (override with `KAVACH_INSTALL_DIR`).
4. **Provisions the enforced Rust toolbelt** (`rg`, `fd`, `bat`, `sd`, `xh`, `jaq`, …) via `kavach toolbelt install` — required, because Kavach's `pre-tool` gate **blocks the legacy POSIX equivalents** (`grep`/`find`/`cat`/`curl`/`sed`) on Linux, macOS, and Windows.
5. **Wires the hooks** — merges Kavach's gate hooks into each AI harness's own `settings.json` (Claude Code, Cursor, Codex, …) via `kavach install --vendor all`; idempotent and backs up the originals, so the lifecycle gates actually fire.
6. **Deletes the clone** — nothing is left behind but the installed binary and its toolbelt.

Your memory store lives in a per-OS data directory the binary resolves automatically: `~/Library/Application Support/SharedAI` (macOS), `%LOCALAPPDATA%\SharedAI` (Windows), `~/.local/share/shared-ai` (Linux).

</details>

<details>
<summary><strong>Update — <code>kavach update</code></strong></summary>

<br>

Updating is a native subcommand (fast, no re-clone burden on you): it clones the latest source, rebuilds, and installs over the running binary.

```bash
kavach update
```

</details>

<details>
<summary><strong>Build manually from source</strong></summary>

<br>

Requires **Rust 1.96+** (edition 2024) and a supported AI harness (Claude Code v2.0+, Codex, or Cursor).

```bash
git clone https://github.com/Wankhede-Brothers/kavach-rs
cd kavach-rs

# Build the release binary (the package is kavach-cli; the binary is `kavach`)
cargo build --release

# Symlink into PATH (Linux/macOS)
ln -sf "$(pwd)/target/release/kavach" ~/.local/bin/kavach

# Provision the enforced Rust toolbelt the gates require
kavach toolbelt install

kavach --version
```

</details>

<details>
<summary><strong>Open the web dashboard (no install)</strong></summary>

<br>

Kavach ships a **server-rendered** web UI (Axum + maud — no desktop app, no webview to install). It reads everything through the running SurrealDB server and renders the memory graph, kanban, decisions, and mistake ledger as plain HTML:

```bash
kavach web
# → serves the dashboard at http://127.0.0.1:7777 (default; override with --port)
```

</details>

<hr>

## 🔌 Wire Into Claude Code

Map each lifecycle event to a Kavach gate. Every gate runs the same way — `kavach gates <name> --hook`, reading the hook JSON from stdin. The config below is the recommended core, kept in sync with [`crates/kavach-cli/templates/harness/claude.settings.json`](crates/kavach-cli/templates/harness/claude.settings.json). Merge its `hooks` block into `~/.claude/settings.json` (user) or `<project>/.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      { "hooks": [ { "type": "command", "command": "kavach gates intent --hook" } ] }
    ],
    "PreToolUse": [
      { "matcher": "Write|Edit|NotebookEdit", "hooks": [ { "type": "command", "command": "kavach gates pre-write --hook" } ] },
      { "matcher": "*",                        "hooks": [ { "type": "command", "command": "kavach gates pre-tool --hook" } ] }
    ],
    "PostToolUse": [
      { "matcher": "Write|Edit|NotebookEdit", "hooks": [ { "type": "command", "command": "kavach gates post-write --hook" } ] },
      { "matcher": "*",                        "hooks": [ { "type": "command", "command": "kavach gates post-tool --hook" } ] }
    ],
    "SessionStart": [
      { "hooks": [ { "type": "command", "command": "kavach gates session-start --hook" } ] }
    ],
    "Notification": [
      { "hooks": [ { "type": "command", "command": "kavach gates notification --hook" } ] }
    ],
    "Stop": [
      { "hooks": [ { "type": "command", "command": "kavach gates stop --hook" } ] }
    ]
  }
}
```

<div align="center">
<table>
<tr><th>Hook event</th><th>Gate</th><th>What it enforces</th></tr>
<tr><td><code>UserPromptSubmit</code></td><td><code>intent</code></td><td>Intent classification + skill routing + harness dispatch + <strong>Brain-OS <code>[RECALL]</code></strong></td></tr>
<tr><td><code>PreToolUse</code> (Write/Edit)</td><td><code>pre-write</code></td><td>Hard enforcement: skills, research, anti-pattern scan</td></tr>
<tr><td><code>PreToolUse</code> (else)</td><td><code>pre-tool</code></td><td>Bash blocklist + read validation + subagent budget</td></tr>
<tr><td><code>PostToolUse</code> (Write/Edit)</td><td><code>post-write</code></td><td>Anti-prod scan + quality + lint + memory sync</td></tr>
<tr><td><code>PostToolUse</code> (else)</td><td><code>post-tool</code></td><td>Context injection + research + <strong>concept harvest</strong> + task sync + <strong>no-fabrication recovery</strong> (a failed <code>kavach</code> call is steered to <code>--help</code>/<code>commands --tree</code>, or <code>just install</code> on a stale binary)</td></tr>
<tr><td><code>SessionStart</code></td><td><code>session-start</code></td><td>Restore state from the DB, not the chat</td></tr>
<tr><td><code>Stop</code></td><td><code>stop</code></td><td>3-witness verify or block + pattern extraction</td></tr>
</table>
</div>

Kavach tracks Claude Code's hook surface as it evolves — the binary also dispatches gates for the wider lifecycle (`pre-compact`/`post-compact`, `session-end`, `worktree-create`/`worktree-remove`, `permission-request`, `elicitation`, `notification`, `message-display`, and more); wire any of them with the same `kavach gates <name> --hook` form.

<details>
<summary><strong>Install the CLI toolbelt</strong></summary>

<br>

Kavach's `pre-tool` gate steers shell commands toward faster Rust equivalents (`grep`→`rg`, `find`→`fd`, `cat`→`bat`, `sed`→`sd`, …). One command provisions the whole set:

```bash
kavach toolbelt install --yes      # fetch all tools via cargo binstall (prebuilt binaries)
kavach toolbelt list               # show each tool, its provider crate, and upstream license
kavach toolbelt install --only rg,fd,bat   # install just a subset
```

It shells out to [`cargo binstall`](https://github.com/cargo-bins/cargo-binstall), pulling each tool's **prebuilt** release binary into your cargo bin directory. No binaries are redistributed inside Kavach; `kavach toolbelt list` surfaces every tool's crate + license for provenance.

</details>

<details>
<summary><strong>Cursor &amp; Codex (same DB, native edges)</strong></summary>

<br>

One Kavach binary and one database serve **all three** harnesses — run Cursor for one task, Codex for another, Claude Code for a third, against a single shared memory bank. Kavach detects which IDE called (by payload shape, or an explicit `--vendor` flag), lowers that harness's native hook payload into its canonical form, runs the vendor-blind gates, and renders the verdict back in each tool's own dialect — including each one's native failure policy (Cursor fails **open** so a hook error never wedges the editor; Codex and Claude Code fail **closed**).

| IDE | Hook config | Install path | Rule file |
|-----|-------------|-------------|-----------|
| Claude Code | `claude.settings.json` | `~/.claude/settings.json` (merge `hooks`) | `CLAUDE.md` |
| Cursor | `cursor.hooks.json` | `~/.cursor/hooks.json` or `<project>/.cursor/hooks.json` | `.cursor/rules/kavach.mdc` |
| Codex | `codex.config.toml` | append to `~/.codex/config.toml` (set `[features] hooks = true`) | `AGENTS.md` |

Cursor has no `SessionStart` event, so Kavach injects the live mistake ledger + rules + kanban into every Cursor turn via its `beforeSubmitPrompt` hook; Codex shares Claude Code's `SessionStart`/`UserPromptSubmit` channel and gets the same context natively. Configs ship under [`crates/kavach-cli/templates/harness/`](crates/kavach-cli/templates/harness/).

</details>

<hr>

## 🧩 Core Concepts

### Gates

Gates are the enforcement primitive. Each gate is a Rust function mapped to a lifecycle event. A gate can **block** (deny the action), **ask** (require confirmation), or **allow** (pass through, optionally injecting context). Detectors live in `kavach-patterns`; the dispatch and severity wiring live in `kavach-engine`.

### Memory (kavach-db)

A SurrealDB store holds durable state across sessions, scoped per project:

- **decisions** — architectural choices, never re-litigated
- **research** — findings from web searches, cached
- **patterns** — gate false-positive fixes, learned over time
- **roadmap** — kanban-style task tracking (the kanban is a status lens over this)
- **app_spec** — six-file project context

All access is RPC-routed through the in-process `kavach-rpc` layer to the SurrealDB server — **no crate opens the database directly**, preserving the single-writer invariant.

### Knowledge Graph

A multi-tier graph: global **concepts** (L0) link to project **entities** (L1), and **mistakes** cluster into anti-patterns by semantic similarity — feeding the same corpus that Brain-OS retrieves over.

### Rules Engine

A dedicated crate family (`kavach-rule-*`) parses, stores, and evaluates rule definitions — the declarative layer that drives skill keyword routing and gate advisories.

### Injection Compaction

Every model-facing gate injection flows through one chokepoint (`compact_inject`) that compresses conversational grammar at Ultra level while preserving code spans, fenced blocks, URLs, `file:line` tokens, `[BRACKET]` signals, and versions **byte-for-byte** — a fail-closed lossless check guards the preservation set. Fewer re-read tokens per turn directly reduces context rot ([Anthropic: effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)); each compaction also records its token savings to the DB fire-and-forget. There is no command to run — compaction is a default property of the gates.

### Action-Imperative Verdicts

A gate denial is a **redirect carrying the next action, never a dead end**. Every emitted verdict is shaped `[KEYWORD] what → do-this → retry` (e.g. `[TOOLBELT_POLICY]`, `[SECRET_CONSUME]`, `[RCA_FIRST]`), and every deny-shaped response appends a `[NEXT_ACTION]` trailer instructing the executor to do the named step and retry — never to report "BLOCKED" or surrender. Gate satisfaction mechanics are reachable from subagent contexts via typed DB rows (e.g. the arch gate auto-injects prior decisions from `arch.list_recent`), so fanned-out workers are never wedged on main-loop-only state.

### Autonomous Harness Loop

Kavach picks the right *agentic workflow shape* for a task and drives it autonomously — no manual orchestration. A task is classified into one of six dynamic-workflow patterns (after Anthropic's [Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents)):

<div align="center">
<table>
<tr><th>Pattern</th><th>Shape</th><th>When</th></tr>
<tr><td><code>classify-act</code></td><td>route → handle</td><td>triage / dispatch-by-type</td></tr>
<tr><td><code>fan-out-synthesize</code></td><td>parallel → merge</td><td>audits, broad sweeps</td></tr>
<tr><td><code>worker-critic</code></td><td>produce → adversarially verify</td><td>reviews, hardening</td></tr>
<tr><td><code>generate-filter</code></td><td>many candidates → keep best</td><td>brainstorming, design search</td></tr>
<tr><td><code>pairwise-tournament</code></td><td>compare → rank → pick winner</td><td>"best of N"</td></tr>
<tr><td><code>loop-until-done</code></td><td>iterate to a goal oracle</td><td>open-ended build/fix (default)</td></tr>
</table>
</div>

The loop is **DB-driven, end to end**: the `intent` gate classifies the prompt and persists the pattern on the next-open roadmap card; the `stop` gate reads that link plus the oracle's last verdict and emits `[AUTO_CONTINUE] run Workflow <path>`. The card's harness link in the DB is the single source of truth, so the choice survives context compaction and session restarts.

<hr>

## 🏗️ Architecture — the crate workspace

The workspace is **20 crates**. The load-bearing ones:

```
kavach-rs/
├── crates/
│   ├── kavach-cli/          # binary entry point (`kavach`) + CLI commands
│   ├── kavach-engine/       # gate dispatch, severity wiring, DAG team scheduler
│   ├── kavach-chain/        # verification chain runner + intent analysis
│   ├── kavach-patterns/     # pattern detectors (the guards gates fire)
│   ├── kavach-rpc/          # in-process JSON-RPC dispatch + client
│   ├── kavach-surreal/      # SurrealDB persistence + Brain-OS retrieval + knowledge graph
│   ├── kavach-session/      # cross-turn session state + mistake ledger
│   ├── kavach-types/        # shared types (HookInput, MemoryStatus, Priority)
│   ├── kavach-config/       # configuration loading
│   ├── kavach-hook/         # harness hook I/O + lifecycle plumbing
│   ├── kavach-advisor/      # advisory client + types
│   ├── kavach-dtree/        # decision-tree primitives (intent classification)
│   ├── kavach-ope/          # order-preserving primitives
│   ├── kavach-toon/         # token-efficient serialization + injection compaction (compact: Lite/Full/Ultra)
│   ├── kavach-web/          # server-rendered web UI (Axum + maud, `kavach web`)
│   └── kavach-rule-*/       # rule ast · parser · engine · generator · storage
└── (skills are loaded from ~/.claude/skills at runtime)
```

<hr>

## 🛠️ Development

Kavach follows a strict-lint, evidence-gated workflow: edition 2024, `unsafe` forbidden workspace-wide, `dead_code` denied, and lib crates use `thiserror` while the app uses `anyhow`.

```bash
cargo check --workspace                      # build all crates
cargo nextest run --workspace                # tests (parallel, per-test process isolation)
cargo clippy --workspace -- -D warnings      # lint (correctness lints deny-by-default)
cargo fmt --all                              # format
just mermaid-check <file.html>               # validate every Mermaid block with mmdc before it ships
```

The **diagram-first** law (injected by the `intent` gate on any design/architecture turn) requires a Mermaid LLD rendered to a temp HTML file *before* deciding. `just mermaid-check` runs that file's diagrams through `mmdc` and exits non-zero on a syntax error — so a broken diagram fails loudly at author time instead of silently rendering as raw text.

The **"done" bar is a 3-witness verify**: an `rg` artifact (the change exists at `file:line`), `git diff --stat` (the diff landed), and `cargo check --workspace` exit 0 (it compiles). The Stop gate blocks until those hold.

<hr>

## 📋 Changelog

Release history lives in [CHANGELOG.md](CHANGELOG.md), newest version first.

<hr>

<div align="center">

## 📜 License

<a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/License-MIT-22863a?style=for-the-badge"></a>

<strong>MIT</strong> © 2026 Wankhede Brothers

<sub>Built in Rust 🦀 — armor for every agent session.</sub>

</div>
