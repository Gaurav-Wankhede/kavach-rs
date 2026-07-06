# Changelog

All notable changes to Kavach are documented here. The newest version is always
at the top. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Kavach versions by **CalVer** (`YY.M.patch` — e.g. `26.7.0`), matching the release tags.

## [26.7.0] — 2026-07-06

Context-rot engineering release: injection compaction blended into every gate,
action-imperative verdict language system-wide, and race-safe mistake-ledger
writes. SOURCE: anthropic.com/engineering/effective-context-engineering-for-ai-agents.

### Added

- **Injection compaction (`kavach-toon::compact`)** — pure compressor with three
  levels (`Lite`/`Full`/`Ultra`); drops conversational grammar from harness
  injections while preserving code spans, fenced blocks, URLs, `file:line`
  tokens, `[BRACKET]` signals, and versions byte-for-byte. `assert_lossless` is
  the fail-closed preservation gate. Measured: prose −32%, structured −22%
  tokens at Ultra.
- **Compaction blended into the gates** — one choke (`kavach-hook::inject::compact_inject`)
  compresses all 17 model-facing `additional_context` emission sites at Ultra
  automatically; no command, no flag. Each compaction fire-and-forgets a
  rot-savings metric through the durable write-spool into the DB
  (`compact.metric.{session}` pattern rows).
- **Arch-gate subagent reachability** — `ArchPreWriteOutcome::AutoInject` loads
  prior typed decisions via `arch.list_recent`, so a subagent executor can
  satisfy the gate through a DB row instead of the main-loop-only skill flag.
  Law: every gate's satisfaction mechanic must be reachable from the executor
  context.
- **Dependency-guess validation** — NLU-harvested `depends_on` edges carry a
  `speculative` provenance flag and must resolve to an existing row or drop
  with a `dep-guess dropped (no such card)` notice; explicit `--depends-on`,
  frontmatter, and wikilink edges pass verbatim. Kills phantom DAG nodes that
  wedged cards into fake-BLOCKED.

### Changed

- **Verdict vocabulary is action-imperative** — `BLOCK`/`BLOCKED` purged from
  every emitted gate message in favor of per-gate keywords
  (`[ARCH_RESEARCH]`, `[SECRET_CONSUME]`, `[DESTRUCTIVE_OP]`, `[TOOLBELT_POLICY]`,
  `[RUST_LAW]`, `[SQL_SAFETY]`, `[RCA_FIRST]`, ~40 strings) shaped
  `[KEYWORD] what → do-this → retry`. All five deny-shaped `HookResponse`
  constructors append an idempotent `[NEXT_ACTION]` trailer ("verdict = redirect,
  not a dead end"). Wire fields, internal identifiers, and regex detection
  literals unchanged.
- **Research gate names its mechanic** — the evidence message states that a
  one-line `// SOURCE: <url>` satisfies both the research gate and the one-line
  comment ceiling, eliminating the false two-gate paradox that made executors
  surrender.
- **RCA template restructured to 5 fields** —
  `symptom@file:line → why-chain→root_cause · class+blast · fix · cite:URL`
  (~40% fewer tokens than the 8-field form), identical across the injection
  template, the `[RCA_FIRST]` deny message, and the shipped directives.
- **`kavach compact` CLI verb removed** — compaction is a default gate behavior,
  not a command; `kavach context` output stays untouched JSON.
- **Transfer-package directives refreshed** — `transfer-package/CLAUDE.md` now
  ships the ultra-compact action-imperative Global Engineering Directives.

### Fixed

- **Mistake-ledger idempotency** — `mistake_event` rows are UPSERTed by a stable
  `blake3(gate·correct·banned·session·turn)` key and the `instance_of` RELATE is
  guarded by an existing-edge check, so an in-session re-file after a partial
  RPC failure converges to one event + one edge instead of inflating the
  recurrence count that ranks `[PRACTICE_DELTA]`.
- **Legacy ledger TOCTOU** — the shell fallback's read-then-branch
  (`--new` vs `--update-key`) race is replaced by `write_with_upsert`
  (update-first, create only on genuine not-found); concurrent writers converge
  to one row.
- **Write-spool double-replay** — `drain()` claims its batch via atomic same-fs
  `fs::rename` before reading; a racing drainer gets NotFound → empty. Each
  spooled line replays exactly once.
- **60+ orphaned tests revived** — `intent_context` (33) and `pre_write_rca_guard`
  (29) test files existed but were never compiled; wired via `#[path]` modules,
  surfacing and fixing one genuinely stale assertion.
- **`kavach-hook` nano-split** — the input-family (`read_hook_input`,
  `parse_hook_input`) moved to a `input.rs` leaf, bringing `lib.rs` under the
  file-size ceiling with re-exports preserving the public API.

## [26.6.0] — 2026-06-03

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

[26.7.0]: https://github.com/Wankhede-Brothers/kavach-rs/releases/tag/26.7.0
[26.6.0]: https://github.com/Wankhede-Brothers/kavach-rs/releases/tag/26.6.0
