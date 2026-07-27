# Kavach-rs Refactoring Plan

**Created:** 2026-07-27
**Status:** PLANNED — awaiting execution
**Scope:** `/Users/gauravwankhede/kavach-rs`

---

## Research Sources

- [Context Engineering for Claude 5](https://claude.com/blog/the-new-rules-of-context-engineering-for-claude-5-generation-models) (July 24, 2026) — "We removed over 80% of Claude Code's system prompt for models like Claude Opus 5 and Claude Fable 5 with no measurable loss"
- [Anthropic SDLC Security](https://claude.com/blog/how-anthropic-secures-its-ai-native-software-development-lifecycle) (July 21, 2026) — "Claude authors about 80% of the code merged into our codebase today"
- [Harness Engineering for Self-Improvement](https://lilianweng.github.io/posts/2026-07-04-harness/) (Lilian Weng, July 4, 2026) — "A harness is every piece of code, configuration, and execution logic that isn't the model itself"
- [Loop vs Harness vs Context Engineering](https://levelup.gitconnected.com/loop-vs-harness-vs-context-engineering-explained-in-detail-268b48ffa2f9) (July 20, 2026) — Four-layer model: Prompt → Context → Harness → Loop
- [Prompt, Context, Harness, Loop: 4 Agent Layers](https://futureagi.com/blog/loop-engineering/prompt-context-harness-loop-layers/) (July 26, 2026) — Layered agent architecture breakdown
- [Loop Engineering DZone](https://dzone.com/articles/loop-engineering-llms) (July 1, 2026) — "Loop engineering is replacing yourself as the person who prompts the agent"

---

## The Problem (Root Cause)

Kavach-rs applies **2025-era harness engineering** (prescriptive rules, nano-file limits, hard blocks) to 2026-era models that have better judgement. The result:

1. **Hard blocks → infinite loops**: LLM hits `Decision::Deny` → retries same operation → hits same deny → loops forever OR bypasses via wrong method
2. **Over-constrained CLAUDE.md**: 112-line template with conflicting rules wastes context tokens on constraints the model no longer needs
3. **Duplicate code**: Two independent blocking systems (`kavach-patterns/checks/blocking.rs` and `kavach-config/blocklist.rs`) with identical `is_inside_quotes` and `blocked_match` functions
4. **20+ CLI commands**: Many conflict or are never used, causing `KAVACH_MISUSE` errors that trigger more loops
5. **Nano-file guard**: Blocks `mod.rs`, enforces 100-LOC ceiling, forces separate test files — all unnecessary for models with tool calling

## The Philosophy

**Four-layer agent architecture (2026):**

| Layer | What it controls | What breaks when weak |
|---|---|---|
| **Prompt** | Wording of one request | Model misreads instruction |
| **Context** | What fills the window | Model lacks facts or drowns in them |
| **Harness** | Tool execution, retries, limits | Failed tool call goes unnoticed |
| **Loop** | When to call again, when to stop | Run repeats forever or quits early |

Kavach-rs currently over-engineers **context** (massive CLAUDE.md, nano-file rules) and under-engineers **harness** (hard blocks without progressive actions) and **loop** (no stop conditions, no progress detection).

**Key insight from Anthropic:** "We removed over 80% of Claude Code's system prompt for models like Claude Opus 5 and Claude Fable 5 with no measurable loss on our coding evaluations."

**Key insight from loop engineering:** "The model's claim that it's finished is worth nothing. A real check is worth everything." — The gate decides done, not the model.

---

## Phase 1: Fix the mod.rs Violation

**Scope:** Single file, zero risk
**Effort:** 5 minutes
**Status:** ✅ COMPLETED (2026-07-27)

### What

The codebase actively BLOCKS `mod.rs` as a P0 violation (see `kavach-patterns/src/nano_file_guard.rs:20-28`). But `kavach-web/src/pages/mod.rs` is the only `mod.rs` in the entire codebase — a violation of the codebase's own guard.

### Action

| File | Action |
|---|---|
| `crates/kavach-web/src/pages/mod.rs` | Rename to `crates/kavach-web/src/pages.rs` |
| `crates/kavach-web/src/pages/` | Keep directory (now sibling to `pages.rs`) |
| `crates/kavach-web/src/lib.rs` or `main.rs` | Update `mod pages;` declaration if path changed |

### Verify

```
cargo check --workspace
cargo test --workspace
```

---

## Phase 2: Remove Nano-File Guard

**Scope:** `kavach-patterns` crate + engine wiring + CLAUDE.md
**Effort:** 30 minutes
**Risk:** Low
**Status:** ✅ COMPLETED (2026-07-27)

### Rationale

Modern LLMs with tool calling (`rg`, `sg`, `fd`) can target exact functions without reading entire files. The 100-LOC ceiling, forced separate test files, and directory depth limits are 2025-era constraints that waste context tokens.

Per the context engineering blog: "Give Claude rules → Let Claude use judgement."

### Action

| File | Action |
|---|---|
| `crates/kavach-patterns/src/nano_file_guard.rs` | Delete module (81 lines) |
| `crates/kavach-patterns/src/nano_file_guard/types.rs` | Delete |
| `crates/kavach-patterns/src/nano_file_guard/predicates.rs` | Delete |
| `crates/kavach-patterns/src/nano_file_guard_test.rs` | Delete |
| `crates/kavach-patterns/src/bin/nano-file-lint.rs` | Delete binary |
| Engine wiring in `pre_write_guards.rs` | Remove `nano_file_guard::detect()` call |
| All `kavach:nano-file-exempt` markers in source | Remove markers |
| `CLAUDE.md` line 63: "Nano-files. One functionality per file..." | Remove nano-files rule |
| `kavach lint audit` / `kavach lint debt` | Remove nano-file metrics |

### Keep (as advisory, not block)

- Directory depth limit (>7) → P2 advisory (not P0)
- Inline test module detection → P1 advisory (not P0)
- LOC warning at 120 → P2 hint (not P0 block)

### Verify

```
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

## Phase 3: Consolidate Duplicate Blocking Systems

**Scope:** `kavach-patterns` + `kavach-config` crates
**Effort:** 1 hour
**Risk:** Medium

### Problem

Two independent blocking systems with identical logic:

| File | Functions |
|---|---|
| `kavach-patterns/src/checks/blocking.rs` | `is_blocked()`, `is_inside_quotes()`, `blocked_match()` |
| `kavach-config/src/blocklist.rs` | `is_blocked_command()`, `is_inside_quotes()`, `blocked_pattern_matches()` |

Both `is_inside_quotes` implementations are **character-for-character identical**: same `(mut sq, mut dq, mut i)` pattern, same escape handling, same byte-slice logic.

`blocked_match` and `blocked_pattern_matches` are semantically the same function with two implementations.

### Action

| File | Action |
|---|---|
| `kavach-config/src/blocklist.rs` | **Canonical home** — single source of truth |
| `kavach-patterns/src/checks/blocking.rs` | **Delete** — redirect callers to `kavach_config::blocklist` |
| `is_inside_quotes()` | Keep ONE in `kavach-config` |
| `blocked_match()` / `blocked_pattern_matches()` | Keep ONE in `kavach-config` |
| `is_blocked()` in kavach-patterns | Replace with re-export from `kavach_config::blocklist` |
| `blocked_defaults()` in `kavach-patterns/config/defaults.rs` | Merge into `kavach-config/src/blocklist.rs` |

### Verify

```
cargo check --workspace
cargo test --workspace
# All existing blocklist tests pass
```

---

## Phase 4: Convert Hard Blocks → Progressive Action Imperatives

**Scope:** `kavach-engine/src/gates/pre_tool_bash/blocklist.rs` + related gates
**Effort:** 2 hours
**Risk:** Medium

### Problem

`Decision::Deny` messages are dead ends. LLM reads "blocked" → retries same operation → infinite loop. Per the context engineering research: "Gate deny = the system handing you the NEXT ACTION. A redirect, never a dead end."

### Current Hard Blocks That Cause Loops

| Location | Current Message | Problem |
|---|---|---|
| `blocklist.rs:35` | `[DESTRUCTIVE_OP]` + fix text | LLM retries same command |
| `blocklist.rs:53` | `[WRITE_BYPASS]` | LLM tries different write method |
| `blocklist.rs:93` | SQL injection block | LLM rephrases query |
| `blocklist.rs:121` | Protected path block | LLM tries different path |
| `legacy_tool_guard` | `[TOOLBELT_POLICY]` | LLM uses `--no-verify` or bypasses |

### Action

Transform each `Decision::Deny` into a **progressive action imperative**:

```rust
// BEFORE (dead end):
Decision::Deny("[DESTRUCTIVE_OP] rm -rf / detected")

// AFTER (progressive action):
Decision::Deny(
    "[DESTRUCTIVE_OP] rm -rf / is irreversible.\n\
     NEXT ACTION: Use `fd` to list target files, then `rm` individual files.\n\
     Example: `fd pattern src/ | head -20` → verify → `rm file1 file2`"
)
```

### Specific Conversions

| Block | Progressive Action |
|---|---|
| `rm -rf` | → "List targets with `fd` first, then `rm` individually" |
| `WRITE_BYPASS` | → "Use the Edit tool instead: `sg` to find the function, `sd` to replace" |
| SQL injection | → "Use parameterized query via `kavach db query-raw`" |
| Protected path | → "Write to a temp file first, then `cp` with approval" |
| Legacy tool | → "Toolbelt policy: grep→rg, find→fd, cat→bat, etc." |
| `chmod 777` | → "Use `chmod 755` for dirs, `chmod 644` for files" |
| `dd to disk` | → "This operation is irreversible. Confirm with user if intentional." |

### Loop-Breaker

After 3 consecutive `Decision::Deny` on the same command pattern, inject `[LOOP_DETECTED]` + forced next action + mistake record.

### Verify

```
cargo check --workspace
cargo test --workspace
# Manual test: trigger each block, verify progressive message appears
```

---

## Phase 5: Slim the CLI — Remove Conflicting/Unused Commands

**Scope:** `kavach-cli/src/cli.rs` + `kavach-cli/src/cmd/` + `kavach-cli/src/cli/db.rs`
**Effort:** 3 hours
**Risk:** High

### Problem

20+ top-level commands and 49 `db` subcommands. Many overlap or are never used. The LLM hits `KAVACH_MISUSE` when it guesses wrong, triggering more loops.

### Proposed CLI After Cleanup

```
kavach
├── status              # Keep — session info
├── servers up/down     # Keep — SurrealDB lifecycle
├── gates <event>       # Keep — hook dispatch
├── session init/end    # Keep — lifecycle
├── db <verb>           # Slim from 49 → ~15 essential verbs
├── install             # Keep — hook installation
├── heal capture/sweep  # Keep — self-healing
├── verify              # Keep — 3-witness check
├── deploy              # Keep — build+install
├── think               # Keep — Brain-OS retrieval
├── toolbelt install    # Keep — Rust CLI provisioning
├── commands --tree     # Keep — help
└── lint init/audit     # Keep — strict profiles
```

### Top-Level Commands to REMOVE or MERGE

| Command | Action | Why |
|---|---|---|
| `kavach ask` | Merge into `think` | Both are retrieval; `think` is more capable |
| `kavach oversized` | Merge into `lint audit` | File size checks belong in linting |
| `kavach doctor` | Merge into `heal sweep` | Both are self-audit |
| `kavach update` | Remove | Use `git pull && cargo build --release` |
| `kavach schema` | Remove | Vendor schema is static, not needed at runtime |
| `kavach loophole sweep/loop/cron` | Merge into `heal` | Self-healing umbrella |
| `kavach pipeline plan/status` | Merge into `phase` | Pipeline is a phase concern |
| `kavach security init/scan/process/report` | Merge into `lint audit` | Security scanning is linting |
| `kavach todos sync` | Merge into `db sync` | Todos are DB entries |
| `kavach tasks audit` | Remove | Claude Code handles this natively |
| `kavach context` | Merge into `status` | Status should include context |
| `kavach mistake` | Merge into `db` | Mistake ledger is DB |
| `kavach bulk` | Merge into `db` | Bulk ops are DB |
| `kavach goal` | Remove | Loop engineering handles this |
| `kavach bg` | Remove | Session handles background work |
| `kavach team` | Remove | Multi-agent is harness concern |
| `kavach rules list/check/generate/show` | Merge into `lint` | Rules are lint profiles |

### `db` Subcommands to KEEP (15)

```
kavach db
├── register            # Project registration
├── query               # Memory retrieval
├── search              # Full-text search
├── write               # Memory write
├── get                 # Single entry fetch
├── kanban              # Board view
├── kanban-close        # Close card
├── status-update       # Update status
├── event               # Event log
├── delete              # Delete entry
├── wipe-project        # DANGER: full wipe
├── gate-config-get/set # Gate overrides
├── concept-add/link/search  # Concept graph
├── citation-add/list   # Citation tracking
├── query-raw           # Direct SurrealQL
└── next-prompt         # Exec prompt output
```

### `db` Subcommands to REMOVE (34)

| Subcommand | Why remove |
|---|---|
| `register-part` | Merged into `register` with `--part` flag |
| `priority-set` | Use `status-update` with priority field |
| `lane-set` | Use `kanban` with lane parameter |
| `sync` | Auto-sync on every `write` |
| `find-project` / `find-part` | Use `query` with path filter |
| `list-projects` / `list-parts` / `tree` | Use `query` with type filter |
| `expire` / `rotate` / `archive` | Auto-cleanup on schedule |
| `populate-graph` / `backfill-relationships` / `graph-query` | Use `concept-*` and `citation-*` |
| `pg-introspect` / `pg-isolation` / `pg-er` / `pg-drift` | PostgreSQL tools — separate binary |
| `delete-prefix` | Use `delete` with wildcard |
| `concept-delete` / `concept-delete-prefix` | Use `delete` with category |
| `citation-get` / `citation-link` / `citation-traverse` / `citation-refresh` | Use `citation-list` + `query-raw` |
| `gate-config-delete` / `gate-config-list` | Use `gate-config-get` + `query-raw` |
| `bridge-create` / `bridge-concepts-for` / `bridge-projects-for` | Use `concept-link` + `query-raw` |
| `mistake-hit-count` / `mistake-purge` | Use `db delete` with category |
| `ope-evaluate` / `ope-audit` | Use `verify` + manual analysis |
| `run-record` / `run-update-status` | Use `event` with run type |
| `flow-add` / `flow-show` / `infer-deps` | Use `write` with flow category |

### Verify

```
cargo check --workspace
cargo test --workspace
# Test every remaining command manually
kavach commands --tree  # Verify tree matches plan
```

---

## Phase 6: Docker vs CLI Decision

**Scope:** Architecture decision — no code changes
**Effort:** 10 minutes (documentation only)

### Current State

SurrealDB runs via `launchd` (macOS) / `systemd` (Linux) as a system service. No Docker files exist.

### Research Finding

Two separate SurrealDB instances must stay separate:
- **kavach-rs**: local `surreal start` on `ws://127.0.0.1:7710` (WebSocket required by Rust client)
- **WebFind**: Docker on `http://localhost:7790` (HTTP for external access)

### Decision: CLI (keep current architecture)

| Factor | Docker | CLI (current) |
|---|---|---|
| **Data preservation** | Risk: volume mapping, migration | Safe: same binary, same path |
| **SurrealDB transport** | HTTP only (Docker networking) | WebSocket (required by `surrealdb` crate) |
| **Complexity** | +Dockerfile, compose.yml, volume mounts | Zero additional files |
| **Performance** | Container overhead | Native performance |
| **Portability** | Needs Docker installed | Single binary |
| **Existing data** | Must migrate `~/Library/Application Support/SharedAI/kavach.surreal` | Already there |

The Rust `surrealdb` crate requires WebSocket transport. Docker's HTTP endpoint doesn't work (proven with WebFind — `http://` connections hang). Keeping CLI with launchd/systemd is the correct architecture.

### Action

Improve `kavach servers up` to auto-detect OS and use the correct service manager (launchd on macOS, systemd on Linux).

---

## Phase 7: Remove Hardcoded Values

**Scope:** Multiple files across crates
**Effort:** 1 hour
**Risk:** Low

| What | Where | Action |
|---|---|---|
| Port `7710` | `servers.rs:14`, `connection.rs:22` | Config via `KAVACH_SURREAL_PORT` env |
| Credentials `root`/`root` | `connection.rs:26-27` | Config via env (already done) |
| Namespace `kavach` / DB `main` | `connection.rs:19-20` | Config via env |
| DB paths per-OS | `connection.rs:31-47` | Keep as defaults, document env overrides |
| Vendor schema URLs | `vendor.rs:268-285` | Move to `kavach-config` constants |
| AI config file names | `bidi_unicode_guard.rs:57-62` | Move to `kavach-config` constants |
| `DEFAULT_PORT: 7777` | `kavach-web/src/lib.rs:23` | Config via `KAVACH_WEB_PORT` env |
| `localhost:3000/api` | `hallucination_guard.rs:16` | Config via env or remove |

### Verify

```
cargo check --workspace
cargo test --workspace
```

---

## Phase 8: Rewrite CLAUDE.md Template

**Scope:** `crates/kavach-cli/templates/harness/CLAUDE.md`
**Effort:** 1 hour
**Risk:** Low

### Current: 112 lines of prescriptive rules (2025-era)
### Target: ~40 lines of progressive action imperatives (2026-era)

### Key Changes

| Current (112 lines) | New (~40 lines) |
|---|---|
| "Nano-files. One functionality per file..." | **Delete** — LLMs use tool calling |
| "One-line comments. Single line = ceiling..." | **Delete** — LLM has judgement |
| "Strict lints, no suppression" (10 lines) | **Keep** — 3 lines: "Run linter. Fix violations. Never suppress." |
| "Permission modes" (4 paragraphs) | **Keep** — 4 lines: one per mode |
| "Turn loop" (7 steps) | **Keep** — 5 lines: condensed |
| "Keywords" (15 prohibitions) | **Keep** — 5 lines: the ones that matter |
| "Gate verdicts = action imperatives" | **Expand** — add progressive action examples for each deny type |
| "RLAIHF" (8 lines) | **Keep** — 3 lines |
| "Production secure system imperatives" (6 items) | **Keep** — 3 lines |
| "exec_prompt" (2 paragraphs) | **Keep** — 3 lines |

### New Section: Loop-Breaker

```markdown
## Loop prevention

If you retry the same operation 3 times and get the same error:
1. Stop retrying
2. Run `kavach commands --tree` to see available verbs
3. Run `kavach <verb> --help` for exact flags
4. If still stuck: `kavach heal capture` → describe the loop → proceed with alternative approach
```

### Verify

```
# Read the new CLAUDE.md
# Verify it's under 50 lines
# Verify all critical rules are preserved
# Verify nano-file rules are removed
# Verify progressive action imperatives are present
```

---

## Execution Order

| Phase | Risk | Effort | Dependencies |
|---|---|---|---|
| 1. Fix mod.rs | Zero | 5 min | None |
| 2. Remove nano-file guard | Low | 30 min | None |
| 3. Consolidate blocking | Medium | 1 hour | None |
| 4. Progressive actions | Medium | 2 hours | Phase 3 |
| 5. Slim CLI | High | 3 hours | Phase 3 |
| 6. Docker decision | Zero | 10 min | None |
| 7. Hardcoded values | Low | 1 hour | None |
| 8. Rewrite CLAUDE.md | Low | 1 hour | Phases 2, 4 |

**Total estimated effort:** ~8 hours

**Data safety:** All changes are code-level refactoring. SurrealDB data at `~/Library/Application Support/SharedAI/kavach.surreal` is untouched. No schema changes. No migration needed.

---

## Verification Checklist

After all phases:

```
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
kavach commands --tree  # Verify slimmed CLI
```

Manual verification:
- [ ] `mod.rs` violation gone
- [ ] Nano-file guard removed
- [ ] Blocking system consolidated
- [ ] Hard blocks show progressive actions
- [ ] CLI commands reduced to ~15 + ~15 db subcommands
- [ ] CLAUDE.md under 50 lines
- [ ] No hardcoded values remain
- [ ] All existing tests pass
- [ ] No SurrealDB data loss
