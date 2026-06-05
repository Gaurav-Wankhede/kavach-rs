# Changelog

All notable changes to Kavach are documented here. The newest version is always
at the top. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Kavach adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-06-03

First public release: a self-improving development harness for Claude Code that
enforces engineering discipline through lifecycle gates, persistent memory, and a
knowledge graph.

### Added

- **Lifecycle gates** — Rust hook engine wired to every Claude Code event
  (`UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `SessionStart`, `Stop`); each
  gate returns allow / block / ask.
- **Persistent memory (kavach-db)** — SurrealDB store, RPC-routed through a
  single-writer daemon: decisions, research, patterns, roadmap, app_spec.
- **Knowledge graph** — three tiers (global concepts L0 → project entities L1 →
  mistakes clustered into anti-patterns L3) using BGE-small embeddings + cosine
  similarity.
- **3-witness verify** — the Stop gate blocks completion until `rg` (exists),
  `git diff --stat` (landed), and `cargo check --workspace` (builds) all pass.
- **Autonomous harness loop** — a task is classified into one of six
  dynamic-workflow patterns (`classify-act`, `fan-out-synthesize`,
  `worker-critic`, `generate-filter`, `pairwise-tournament`, `loop-until-done`)
  and driven end to end, DB-driven:
  - intent gate classifies the prompt and persists the pattern on the next card
    (`db.set_harness`);
  - roadmap cards carry `harness` + `workflow_path` columns;
  - the Stop gate reads the link (`db.get_harness`) and the oracle's last verdict
    (`db.latest_goal_attempt`) to emit `[AUTO_CONTINUE] run Workflow <path>`;
  - `kavach goal compile` turns a tagged-enum `loop.yaml` into a Claude Code
    Workflow.
- **Rules engine** (`kavach-rule-*`) — declarative skill keyword routing and gate
  advisories.
- **Toolbelt enforcement** — gate blocks legacy POSIX tools in favor of the Rust
  CLI toolbelt (rg/fd/bat/eza/sd/…).
- **Desktop / web UI** (`kavach-app`) — knowledge-graph viewer with live updates.
- **Transfer package** — one-shot install bundle (`transfer-package/`) carrying
  the Global Engineering Directives, settings, agents, rules, and commands.

### Fixed

- Pre-write `effective_content` now reconstructs the true post-edit body
  (`old_string` → `new_string`) for Edit, so LOC-exempt-marker and hub-split
  checks judge the result rather than stale content.
- Micro-file split gate now also fires on Edit/Update, not only Write.

### Security

- `unsafe` forbidden workspace-wide; strict-lint contract (edition 2024,
  `dead_code` denied, clippy correctness lints deny-by-default).
- Destructive shell operations (`rm -rf`, etc.) blocked or asked at the
  `PreToolUse` gate; fail-closed on uncertainty.

[0.1.0]: https://github.com/Wankhede-Brothers/kavach-rs/releases/tag/v0.1.0
