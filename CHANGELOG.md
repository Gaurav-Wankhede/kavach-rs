# Changelog

All notable changes to Kavach are documented here. The newest version is always
at the top. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Kavach follows **Semantic Versioning** (`MAJOR.MINOR.PATCH` — e.g. `1.3.4`), matching Cargo package versions and release tags.

## [1.3.5] - 2026-10-12

### Added
- **`kavach replace` Git-Style Unified Diff Telemetry**: In-place replacement now renders standard `-` (red) and `+` (green) lines with 1-indexed line and column coordinates, whitespace indentation analysis (`indent: 4 spaces`, `1 tab`, etc.), and visual multi-line context blocks for both live executions and `--dry-run`.
- **`kavach cargo test` Output Defense**: Clean non-interactive testing that reports clean success summaries with passed/ignored counts and test-suite totals (`[CARGO:TEST] ok: X passed (across Y test suites)`), selectively isolating only failing test blocks and errors on failure.

### Removed
- **`crates/kavach-cli/templates`**: Completely excised all legacy harness template files and dead template integration tests.
- **`kavach install` Subcommand**: Removed obsolete install command in favor of standard single-command installation via `cargo install kavach-cli`.

## [1.3.4] - 2026-10-12

### Added
- **`kavach script`**: Sovereign in-memory pseudo-code scripting engine with zero compiler overhead powered by Rhai. Includes pre-bound live web discovery probes (`api_llms_txt`, `api_sitemap`, `api_openapi`, `http_get`, `http_get_json`, `html_select`) and local file helpers (`fs_read`, `fs_write`, `fs_lines`, `fs_exists`, `hash_blake3`).
- **`kavach cargo`**: Context-defending compiler proxy that automatically routes `kavach cargo test` to `cargo nextest run` with compiler scroll and banner suppression.
- **`kavach grep` Formatter**: Clean deduplicated single file headers with 4-space tab-indented matching lines (`    <line>\t<code>`).
- **Full-Production SaaS Commercial Video**: Authored in 1080p landscape adhering to YouTube motion doctrines, GSAP 3.15.0, and Tailwind CSS v4.

## [1.3.3] - 2026-10-11

### Changed
- Refactored in-process Git suite to 100% Pure-Rust `gix` v0.89.

## [1.3.2] - 2026-10-11

### Added
- **`kavach yaml`**: Dot-path projection to clean TOON v4.4 (`-k` bare keys, `-r` raw scalar) and safe atomic in-place mutation via `--set`.
- **`kavach toml`**: TOML table and array projection to TOON v4.4 and safe atomic in-place mutation via `--set`.
- **`kavach env`**: Zero-Secret Leakage Environment Variable Manager. Values after `=` are permanently masked as `[SET]` to prevent credential exfiltration into LLM context; supports `-k` bare keys, Tree-Sitter AST code-to-env drift auditing via `--diff`, and safe in-place key renaming via `--rename` (preserving secret values byte-for-byte).
- **`kavach html`**: High-precision HTML DOM extraction powered by `scraper` and CSS Selectors (`--attr` attribute value, `-t` text stripping, `-c` element count).
- **`kavach svg`**: Specialized 2D vector DOM inspection and manipulation powered by the `svg` crate (path coordinate extraction `--paths`, in-place palette swapping `--recolor`, and viewport scaling `--view-box`).
- **Atomic `--set` in `kavach json`**: Safe in-place mutation with syntax pre-validation and tempfile atomic swapping.

## [1.3.1] - 2026-10-11

### Removed
- **`kavach slice`**: Permanently purged to eliminate sliding-window and chunk paging context spam. Zero whole-file reads strictly enforced.

### Changed
- **Strict AST Boundary in `kavach grep`**: Arbitrary text search is now strictly forbidden in executable code files (`.rs`, `.ts`, `.py`, etc.) and returns exit code 2. Code files require AST semantic queries (`--def`, `--call`, `--type`, or AST pattern with `--scope`). Arbitrary text patterns are strictly reserved for documentation and prose files (`.md`, `.txt`).
- **Rust Toolchain Modernization**: Upgraded `rust-toolchain.toml` to tracking `channel = "stable"` (resolving to Rust 1.99.0).
- **Workspace Dependency Upgrades**:
  - `ast-grep-core` upgraded to `0.50.0`
  - `ast-grep-language` upgraded to `0.50.0`
  - `jsonrpsee` upgraded to `0.26.1`
  - `dirs` upgraded to `7.0.0`
  - `itertools` upgraded to `0.15.0`
  - `maud` upgraded to `0.27.0`
  - `tower-http` upgraded to `0.7.1`

## [1.3.0] — 2026-10-11

Sovereign agent proxy suite (`kavach grep`, `kavach trace`, `kavach json`, `kavach slice`, `kavach glob`, `kavach replace`) for anti-context-rot agentic workflows, zero-heap path matching, polyglot semantic AST code search across 28 Tree-Sitter grammars, Tab-indented TOON dependency graphs with Unix colon locations, zero-bloat state projections replacing `jaq`/`jq`, and full `webfind` live grounding across all gates.

### Added

- **`kavach grep`** — Sovereign polyglot AST code search engine across 28 languages via Tree-Sitter (`--def <NAME>`, `--call <NAME>`, `--type <NAME>`). Automatically ignores comments and string literals in code to eliminate false positives, while routing documentation/prose files to fast byte/word matching. Features `-s, --scope` to extract entire enclosing AST declarations in 1 turn and outputs clean, grouped Unix colon lines (`line:col:<exact_code>`).
- **`kavach trace`** — Symbol context network and blast-radius inspector. Discovers definitions, inbound callers, and outbound dependencies formatted in a token-optimized, Tab-indented TOON hierarchy with standard clickable Unix `path:line:col` locations (saving >65% tokens vs JSON/ASCII trees).
- **`kavach json`** — Sovereign JSON state and configuration projector. Navigates dot-path queries (`.field[0].sub`), emitting official TOON v4.4 tabular representations (`[N]{fields}:`), bare unquoted keys (`-k`), or raw scalar strings (`-r`), completely replacing `jaq` and `jq`.
- **`kavach slice`** — Zero-bloat surgical line-range reader (`kavach slice <path> <start>:<end>`) for streaming exact line ranges without whole-file reads or heap buffering.
- **`kavach glob`** — Sovereign agent proxy for `fd`/`find`. Zero-heap path matcher using `ignore::WalkBuilder` and `glob_match`. Gitignore-aware, normalizes path prefixes by stripping leading `./`, and supports `-t/--type` (file/dir), `-H/--hidden`, `-d/--max-depth`, and `-m/--max-count`.
- **`kavach replace`** — Sovereign agent proxy for `sd`. High-throughput in-place atomic string and multi-pattern replacement via `aho_corasick` and atomic file persistence (`tempfile`). Includes `--dry-run` safety inspection and match limits.

### Changed

- **Purged Command Aliases** — Removed `graph` and `network` aliases from `kavach trace` to enforce a single canonical command.
- **Trace Limits Uncapped** — Removed hardcoded truncation limits (`take(15)`) in `kavach trace` to emit complete caller/dependency networks.
- **Live Internet Grounding Across All Gates** — Replaced legacy model hallucination risks and generic web search directives across `kavach-engine` and `kavach-patterns` with explicit `webfind search --live "<query>"`.
- **Cross-Language AST Safety** — `kavach grep` gracefully compiles language patterns via `Pattern::try_new(pattern, lang)`, skipping unparseable grammars during multi-language directory walks rather than panicking.

## [26.7.1] — 2026-07-21

Context-rot hardening release: six-file gate code fully removed, SurrealDB→LLM pipeline compressed, global anti-loop enforcement, and session-start injection diet.

### Added

- **SurrealDB→LLM context compression** — `kavach-surreal::context_compress` module with `compress_db_json_string()` and `compress_db_rows()`: every Brain-OS memory row and DB query result bound for agent context is compacted (dedup keys, drop nulls, collapse arrays, trim whitespace) before injection. Kills the primary context-rot vector from verbose DB payloads.
- **Pre-compact / post-compact gates** — lifecycle hooks that compress context before it reaches any PreGate (pre-write, pre-tool, intent) and after PostToolUse, ensuring the LLM never sees uncompressed DB output.
- **Global anti-loop rule** — `pre_tool.rs` tracks the last N tool calls per session; a repeated identical command (same tool + same arguments) triggers `[LOOP_DETECTED]` → forces a root-cause check (`kavach doctor` / `kavach audit`) before the tool is allowed to fire again. Prevents the "same edit retry" death spiral.
- **Edit staleness guard** — `pre_tool_edit_guard.rs` intercepts `Edit` tool calls where `old_string` no longer matches the on-disk file (detected via `READ_HASH_MISMATCH`), emits `[EDIT_STALE] Re-read file → retry with current content`, and blocks the edit. Eliminates the `old_string not found` loop class.
- **`dm_truncate()`** — Mermaid diagram labels in roadmap DAG output are truncated to 60 characters, preventing oversized Mermaid blocks from inflating context.

### Changed

- **Six-file gate code fully removed** — the orphaned `gates/six_file/` directory (23 files) and `gate_runner/six_file_gates.rs` dispatcher deleted. `kavach-types::six_file` renamed to `spec_types` (preserving `ProjectTier`, `FOURTEEN_PREFIXES`, `SpecCategory`, `AutoDraftSource` for CLI). The six-file skill is no longer referenced. Projects now rely on Brain-OS `auto_query_memory()` + compressed DB rows + SurrealDB graph traversal.
- **Research gate severity: P0 → P1** — missing research evidence now emits an advisory (`[RESEARCH:PENDING]`) instead of a hard block, allowing the agent to proceed while surfacing the citation gap. The advisory still demands a `// SOURCE: <url>` line on the next write.
- **AUTONOMY_CONTRACT compressed 77%** — `crates/kavach-engine/src/gates/session_start/context.rs` line 14: the ~1,200-byte multi-line session-start contract is now a single 280-byte imperative line. Same semantics, fewer tokens injected every session start.
- **Context Compression Engine default: Ultra** — all gate injections now flow through `compact_inject` at Ultra level (grammar stripped, code/URLs/file:line preserved byte-for-byte) as a default property, not an opt-in.

### Fixed

- **Loop-on-denial bug** — when a gate denial (e.g., `[RESEARCH_EVIDENCE]`) was triggered, the executor previously re-ran the same tool call verbatim instead of acting on the verdict's `[NEXT_ACTION]` trailer. The anti-loop guard now detects this pattern and forces a corrective action before retry.
- **Context rot from SurrealDB interactions** — DB query results (kanban cards, decisions, patterns) were injected raw into LLM context, consuming 400–2,000 tokens per query. Now compressed at the source before gate injection.

## [1.2.0] — 2026-10-07

CLI surface hardening, `mistake` corpus integration into hybrid recall, and full noise purge.

### Install

```bash
# Install latest from crates.io
cargo install kavach-cli

# Or install from source
cargo install --git https://github.com/gauravwankhede/kavach-rs --bin kavach --locked
```

### Added

- **`kavach db mistakes`** — Anti-pattern ledger promoted from top-level `kavach mistake` into the unified DB namespace (`kavach db mistakes list|inspect|stats|record|clear|clear-all`). All six subcommands available.
- **Mistake corpus in hybrid vector search** — `CORPUS_CATEGORIES` in `kavach-db::turso::search` expanded from 5 → 6; `"mistake"` entries now participate in BM25 ∪ Vector ∪ Graph RRF retrieval via `kavach think`.
- **Auto-recall `keep_hit` allowlist** — `kavach-engine::gates::intent::recall` now auto-surfaces mistake entries on every pre-action prompt (added `"mistake"` to `keep_hit` pattern list). Anti-patterns fire before any destructive tool call.
- **`kavach session state`** — `WorkingStateAction` wired under `SessionAction::State` for clean turn-state access via `kavach session state get|set|clear|inject`. Removes the need for the hidden `kavach working-state` alias.

### Changed

- **CLI surface consolidated 24 → 10 visible commands** — the following were hidden or purged from the public surface:

  | Command | Disposition | Replacement |
  |---|---|---|
  | `kavach loop` | Hidden | Internal only |
  | `kavach heal` | Hidden | Internal only |
  | `kavach loophole` | Hidden | Internal only |
  | `kavach phase` | **Purged** | `kavach verify` covers SDLC gating |
  | `kavach security` | Hidden | `kavach audit --lens security` |
  | `kavach lint` | Hidden | `kavach audit --lens yagni` |
  | `kavach pipeline` | Hidden | `kavach gates` |
  | `kavach context` | Hidden | `kavach session state` |
  | `kavach working-state` | Hidden | `kavach session state` |

- **Final visible CLI surface (10 commands)**:

  ```
  kavach status        Session health, build identity, enforcement flags
  kavach servers       Background daemon lifecycle (Turso DB + Web UI)
  kavach gates         AI agent firewall (pre-tool|post-tool|intent|stop)
  kavach session       Lifecycle + turn state (init, resume, end, state)
  kavach db            Single Source of Truth (kanban, mistakes, memory, graph, vectors)
  kavach install       Hook config installer (Claude Code / Cursor / Codex)
  kavach audit         Multi-lens auditor (bug|security|attack|yagni)
  kavach origin        <5ms AST/LSP symbol resolver (file:line)
  kavach verify        Zero-trust gate: cargo test + clippy → done→verified
  kavach think         3-Signal Hybrid Retrieval (BM25 ∪ Vector ∪ Graph → RRF)
  ```

### Fixed

- `kavach mistake` (top-level) removed — prevented namespace collision with `kavach db mistakes`.
- `MistakesAction` enum rename: all three references in `mistakes.rs` updated (was `MistakeAction`).
- `WorkingStateAction` visibility corrected to `super::WorkingStateAction` path in `cli/session.rs`.

---

## [1.1.0] — 2026-10-06

### Added
- **Native Parallel AST & Search Engine (`kavach-engine::ast`)**:
  - In-process, gitignore-aware parallel source file traversal using `ignore::WalkBuilder` (`ast::walker`).
  - Native `ast-grep-core` tree-sitter AST symbol querying with pre-filter short-circuiting (`ast::scan`).
- **Sub-Millisecond Fast-Path Gate Bypass (`kavach-hook`)**:
  - In-memory fast-path approval for read-only actions and developer queries (<1ms execution without DB lock acquisition).
- **Tri-Modal Embedded Database Schema**:
  - Native vector embeddings with `F32_BLOB(768)` and cosine index on `graph_nodes`.
  - DAG edge constraints on `graph_edges` and daemonless registration on `expert_capabilities`.

### Changed
- **CLI Surface Consolidation**:
  - Pruned obsolete subcommands (`kavach hunt`, `kavach verify-frontend`).
  - Unified multi-lens auditing under `kavach audit` (`--lens`, `--deep`, `--fix-cards`).
  - Compressed `kavach context` into single-line high-density `[STATE]` badges.
- **Dependency Noise Elimination**:
  - Purged dead dependencies from `kavach-toon` (`serde`) and `kavach-patterns` (`kavach-config`).
  - Cleaned quality gates (`cargo machete` exiting 0 with zero unused dependencies).

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

[26.7.0]: https://github.com/Gaurav-Wankhede/kavach-rs/releases/tag/26.7.0
[26.6.0]: https://github.com/Gaurav-Wankhede/kavach-rs/releases/tag/26.6.0
