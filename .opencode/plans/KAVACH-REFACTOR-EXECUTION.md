# Kavach-rs Refactoring — Execution Plan (All Pending Phases)

**Created:** 2026-07-27
**Status:** PLANNED — awaiting execution
**Scope:** `/Users/gauravwankhede/kavach-rs`
**Parent:** `KAVACH-REFACTOR-PLAN.md`

---

## Phase 3: Consolidate Duplicate Blocking Systems

**Effort:** 1 hour | **Risk:** Medium

### Problem

Two independent blocking systems with identical logic:

| File | Lines | Functions |
|---|---|---|
| `kavach-patterns/src/checks/blocking.rs` | 18 | `is_blocked()` — thin duplicate |
| `kavach-config/src/blocklist.rs` | 306 | `is_blocked_command()`, normalization, 10+ blocking functions |

Both use `kavach_config::blocking_util::blocked_pattern_match`. The patterns version loads its own config and only does basic `is_blocked(cmd)`.

### Investigation Needed

Before removing `kavach-patterns/blocking.rs`, check:

```bash
# Is blocking::is_blocked used anywhere?
rg "blocking::is_blocked" --type rust
rg "kavach_patterns::checks::blocking" --type rust
rg "checks::blocking" --type rust
```

### Action

| Step | File | Action |
|---|---|---|
| 1 | `kavach-patterns/src/checks/blocking.rs` | Check usage via `rg` above |
| 2 | If unused | Delete file |
| 3 | If used | Replace callers with `kavach_config::blocklist::is_blocked_command()` |
| 4 | `kavach-patterns/src/checks/mod.rs` | Remove `mod blocking;` declaration |
| 5 | `kavach-patterns/src/checks/` | Remove any re-exports |

### Verify

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

## Phase 4: Convert Hard Blocks → Progressive Action Imperatives

**Effort:** 2 hours | **Risk:** Medium

### Problem

`Decision::Deny` messages in `kavach-engine/src/gates/pre_tool_bash/blocklist.rs` are dead ends. LLM reads "blocked" → retries same operation → infinite loop.

### Files to Modify

| File | Action |
|---|---|
| `kavach-engine/src/gates/pre_tool_bash/blocklist.rs` | Transform Deny messages |
| `kavach-engine/src/gates/pre_tool_bash/write_bypass.rs` | Transform Deny messages |
| `kavach-engine/src/gates/pre_tool_bash/bulk_op_guard.rs` | Transform Advisory messages |

### Specific Conversions

| Current Block | Progressive Action |
|---|---|
| `[DESTRUCTIVE_OP] rm -rf / detected` | → "List targets with `fd` first, then `rm` individually: `fd pattern src/ \| head -20` → verify → `rm file1 file2`" |
| `[WRITE_BYPASS]` | → "Use the Edit tool instead: `sg` to find the function, `sd` to replace" |
| SQL injection block | → "Use parameterized query via `kavach db query-raw`" |
| Protected path block | → "Write to a temp file first, then `cp` with approval" |
| `[TOOLBELT_POLICY]` legacy tool | → "Toolbelt policy: grep→rg, find→fd, cat→bat, sed→sd, curl→xh, jq→jaq" |
| `chmod 777` | → "Use `chmod 755` for dirs, `chmod 644` for files" |
| `dd to disk` | → "This operation is irreversible. Confirm with user if intentional." |
| `psql` blocked | → "Use `kavach db query-raw --query 'SELECT ...'` instead" |
| `sqlx migrate` without RCA | → "Run `kavach heal capture --incident <name>` first, then retry" |

### Loop-Breaker

After 3 consecutive `Decision::Deny` on the same command pattern:
1. Inject `[LOOP_DETECTED]` prefix
2. Add forced next action
3. Record mistake via `kavach mistake record`

### Verify

```bash
cargo check --workspace
cargo test --workspace
# Manual test: trigger each block, verify progressive message appears
```

---

## Phase 5b: Remove Overlapping Commands

**Effort:** 2 hours | **Risk:** High

### Commands to REMOVE

| Command | Action | Why |
|---|---|---|
| `kavach ask` | **Delete** | Not relevant to the system |
| `kavach doctor` | **Merge into `heal sweep`** | Both are self-audit |
| `kavach oversized` | **Merge into `lint audit`** | File size checks belong in linting |
| `kavach pipeline plan/status` | **Merge into `phase`** | Pipeline is a phase concern |
| `kavach security init/scan/process/report` | **Merge into `lint audit`** | Security scanning is linting |
| `kavach todos sync` | **Merge into `db`** | Todos are DB entries |
| `kavach context` | **Merge into `status`** | Status should include context |
| `kavach mistake` | **Merge into `db`** | Mistake ledger is DB |
| `kavach bulk` | **Merge into `db`** | Bulk ops are DB |
| `kavach goal` | **Remove** | Loop engineering handles this |
| `kavach loophole sweep/loop/cron` | **Merge into `heal`** | Self-healing umbrella |
| `kavach rules list/check/generate/show` | **Merge into `lint`** | Rules are lint profiles |

### `kavach ask` Removal (3 files)

**IMPORTANT:** `kavach_advisor` crate is used by engine gates (research_consume, directive_cache, intent). Only remove the CLI command, NOT the crate.

| File | Action |
|---|---|
| `crates/kavach-cli/src/cmd/ask.rs` | Delete entire file (19 lines) |
| `crates/kavach-cli/src/cmd.rs` | Remove `mod ask;` (line 2) + `Commands::Ask` dispatch arm (line 71) |
| `crates/kavach-cli/src/cli.rs` | Remove `Commands::Ask` variant (lines 138-144) |
| `crates/kavach-cli/templates/harness/CLAUDE.md` | Remove `kavach ask` from command tree |
| `crates/kavach-advisor/` | **KEEP** — used by engine gates (research_consume, directive_cache, intent) |

### `kavach doctor` → `heal sweep`

| File | Action |
|---|---|
| `crates/kavach-cli/src/cmd/doctor.rs` | Move logic into `heal/sweep.rs` |
| `crates/kavach-cli/src/cmd.rs` | Remove `Doctor` dispatch arm |
| `crates/kavach-cli/src/cli.rs` | Remove `Commands::Doctor` variant |

### `kavach oversized` → `lint audit`

| File | Action |
|---|---|
| `crates/kavach-cli/src/cmd/oversized/` | Move logic into `lint/audit.rs` |
| `crates/kavach-cli/src/cmd.rs` | Remove `Oversized` dispatch arm |
| `crates/kavach-cli/src/cli.rs` | Remove `Commands::Oversized` variant |

### Other Merges

| Source | Target | Action |
|---|---|---|
| `pipeline plan/status` | `phase` | Add `phase plan` and `phase status` subcommands |
| `security *` | `lint audit` | Add `--security` flag to `lint audit` |
| `todos sync` | `db` | Add `db todos-sync` subcommand |
| `context` | `status` | Merge context JSON into `status` output |
| `mistake` | `db` | Add `db mistake-*` subcommands |
| `bulk` | `db` | Add `db bulk-*` subcommands |
| `loophole *` | `heal` | Add `heal loophole-*` subcommands |
| `rules *` | `lint` | Add `lint rules-*` subcommands |
| `goal` | Remove | Loop engineering handles this |

### Verify

```bash
cargo check --workspace
cargo test --workspace
kavach commands --tree  # Verify removed commands are gone
```

---

## Phase 5c: Slim DB Subcommands

**Effort:** 3 hours | **Risk:** High

### Current: 62 `db` subcommands

### Target: ~15 essential `db` subcommands

### DB Subcommands to KEEP (15)

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

### DB Subcommands to REMOVE (47)

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
| `flow-add` / `flow-show` / `infer-d eps` | Use `write` with flow category |
| `set-parent` | Use `register` with `--parent` flag |

### Verify

```bash
cargo check --workspace
cargo test --workspace
kavach commands --tree  # Verify slimmed CLI
```

---

## Execution Order

| Phase | Risk | Effort | Dependencies |
|---|---|---|---|
| 3. Consolidate blocking | Medium | 1 hour | None |
| 4. Progressive actions | Medium | 2 hours | Phase 3 |
| 5b. Remove overlapping | High | 2 hours | None |
| 5c. Slim db subcommands | High | 3 hours | Phase 5b |

**Total estimated effort:** ~8 hours

**Data safety:** All changes are code-level refactoring. SurrealDB data at `~/Library/Application Support/SharedAI/kavach.surreal` is untouched. No schema changes. No migration needed.

---

## Verification Checklist

After all phases:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
kavach commands --tree  # Verify slimmed CLI
```

Manual verification:
- [ ] `kavach ask` removed
- [ ] Duplicate blocking consolidated
- [ ] Hard blocks show progressive actions
- [ ] CLI commands reduced to ~15 + ~15 db subcommands
- [ ] No SurrealDB data loss
