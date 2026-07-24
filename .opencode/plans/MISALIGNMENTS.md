# Kavach-RS Misalignments & Action Imperatives

**Generated:** 2026-07-24 (v2 - Rust toolbelt scan)
**Scope:** Gate blocks, context rot, SurrealDB/graph, error handling, concurrency, hardcoded values
**Total Issues:** 49

---

## Executive Summary

| Category | Critical | High | Medium | Low | Dead | Total |
|----------|----------|------|--------|-----|------|-------|
| A. Gate Blocks | 0 | 1 | 5 | 4 | 0 | 10 |
| B. Context Rot | 0 | 1 | 3 | 7 | 0 | 11 |
| C. SurrealDB/Graph | 2 | 0 | 0 | 3 | 3 | 12 |
| D. unwrap/expect | 0 | 0 | 4 | 3 | 0 | 7 |
| E. Error Swallowing | 0 | 2 | 3 | 2 | 0 | 7 |
| F. Concurrency | 0 | 0 | 3 | 2 | 0 | 5 |
| G. Hardcoded Values | 0 | 0 | 3 | 5 | 0 | 8 |
| H. Lint Suppressions | 0 | 0 | 0 | 4 | 0 | 4 |
| I. Hardcoded Paths | 0 | 0 | 2 | 1 | 0 | 3 |
| J. Dead Code | 0 | 0 | 0 | 0 | 3 | 3 |
| **Total** | **2** | **4** | **23** | **31** | **3** | **49** |

---

## A. Gate Blocks / Overly Aggressive Denials

### A1. Context Compression Score Keyword Mismatch

**Severity:** High
**File:** `crates/kavach-engine/src/gates/context_compress.rs:84-91`
**Issue:** Score keywords `[hot_patterns]` don't match actual injected header `[SELF_EVOLVE_PATTERNS]`.
**Impact:** Hot patterns get default score 50 instead of intended 30, consuming compression budget.
**Action Imperative:** Add `[SELF_EVOLVE_PATTERNS]` to the score 30 band.

### A2. TDD Guard Misleading Severity Language

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/pre_write_guards/tdd_guard.rs:31-62`
**Issue:** Guard messages say "P0" but engine routes as advisory.
**Action Imperative:** Change guard messages to say "advisory" not "P0".

### A3. Budget Downgrade Silently Suppresses P0Block Gates

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/router/dispatch.rs:22-38`
**Issue:** Budget-exceeded gates silently downgrade to `exit_silent()`.
**Action Imperative:** Emit warning log when budget downgrade occurs.

### A4. RPC Failure Masquerades as Empty Done Queue

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/stop_dispatch/verify.rs:41-60`
**Issue:** `.ok().and_then(...)` returns `Vec::new()` on RPC error.
**Action Imperative:** Distinguish RPC error from empty result.

### A5. Research Circuit Breaker Force-Allows After 3 Blocks

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/pre_write_guards/research_consume.rs:45-92`
**Issue:** Force-allows after 3 blocks on same file.
**Action Imperative:** Increase threshold to 5 or require mistake-record.

### A6. Evidence Detection Is String-Matching

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/pre_write_guards/research_consume.rs:143-149`
**Issue:** `http://` in a comment satisfies gate.
**Action Imperative:** Require evidence in non-comment context.

### A7. Done Gaming Emoji Detection Is Raw Line-Based

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/stop/done_gaming.rs:53-62`
**Issue:** Legitimate status reports flagged as gaming.
**Action Imperative:** Require gaming language in code comments only.

### A8. Silent IO Guard Blanket-Blocks `let _ = fn_call`

**Severity:** Low
**File:** `crates/kavach-patterns/src/silent_io_guard.rs:63-67`
**Issue:** `writeln!` best-effort logging caught.
**Action Imperative:** Exempt `writeln!`, `write!`, `println!`, `eprintln!`.

### A9. Path-Based Backend Detection Is Over-Broad

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/pre_write_response_guard/block.rs:11-22`
**Issue:** Any `.rs` file with "handler" in path treated as backend.
**Action Imperative:** Require `src/handlers/` or `src/routes/` directory.

### A10. Duplicate Function Guard Jaccard Threshold Too Aggressive

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/pre_write_guards/duplicate_function_guard/decision.rs:26-27`
**Issue:** 0.85 threshold triggers on boilerplate.
**Action Imperative:** Increase to 0.90 or exempt common boilerplate.

---

## B. Context Rot

### B1. Score Keyword Mismatch (Duplicate of A1)

**Severity:** High
**File:** `crates/kavach-engine/src/gates/context_compress.rs:84-91`

### B2. Per-Section Budgets Are Additive, Not Global

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/session_start/context.rs:12`
**Issue:** 1200B cap per section, total 10-12KB before compression.
**Action Imperative:** Implement global token budget.

### B3. Memory Bank Ancestry Walk Is O(N) Sequential RPC

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/session_start/memory.rs:14-31`
**Issue:** 2*N RPC calls per ancestor, no depth cap.
**Action Imperative:** Cap ancestry depth at 3, parallelize RPC.

### B4. Sequential FTS Queries Across 6 Tables

**Severity:** Medium
**File:** `crates/kavach-surreal/src/brain_query.rs:73-79`
**Issue:** 6 sequential async FTS queries.
**Action Imperative:** Parallelize via `futures::join_all`.

### B5. Hardcoded LIMIT 25 Over-Fetches 5-10x

**Severity:** Low
**File:** `crates/kavach-surreal/src/brain_query.rs:42`
**Action Imperative:** Scale per-corpus limit based on final limit.

### B6. Token Budget Uses i32

**Severity:** Low
**File:** `crates/kavach-session/src/state/struct.rs:36-37`
**Action Imperative:** Migrate to u64.

### B7. Dead Code in context_compress.rs

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/context_compress.rs:5-6`
**Action Imperative:** Remove unused `HARD_CAP_TOKENS`.

### B8. Redundant Module Injection Calls

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/post_compact.rs:50`
**Action Imperative:** Use HashSet for modules_injected.

### B9. No Output Trimming for Read/Grep

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/post_tool/trim.rs:18-22`
**Action Imperative:** Add trimming for Grep and Read.

### B10. Orphaned schema_v2 Tables

**Severity:** Low
**File:** `crates/kavach-surreal/src/schema_v2.rs:1-146`
**Action Imperative:** Delete file and remove export.

### B11. Legacy Fallback Spawns 6 CLI Subprocesses

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/session_start/patterns/mistakes.rs:32-35`
**Action Imperative:** Batch all keys in single CLI call.

---

## C. SurrealDB / Graph Memory Misalignments

### C1. BEGIN/COMMIT Breaks Concept Edges on WS Engine

**Severity:** Critical
**File:** `crates/kavach-surreal/src/graph/concepts/relate.rs:41-56`
**Issue:** LET-bound ids become NONE inside txn, edges silently not written.
**Action Imperative:** Remove BEGIN/COMMIT, use separate queries.

### C2. Concept FTS Query Missing BM25 Scoring

**Severity:** Critical
**File:** `crates/kavach-surreal/src/graph/concepts/query.rs:45-47`
**Issue:** No `search::score()` or `ORDER BY`.
**Action Imperative:** Add scoring and ordering.

### C3. Dead schema_v2.rs

**Severity:** Dead
**File:** `crates/kavach-surreal/src/schema_v2.rs:1-146`
**Action Imperative:** Delete file.

### C4. Dead schema_engine.surql

**Severity:** Dead
**File:** `crates/kavach-surreal/src/schema_engine.surql:1-53`
**Action Imperative:** Wire into startup or consolidate.

### C5. list_edges_among Issues N*32 Queries

**Severity:** Performance
**File:** `crates/kavach-surreal/src/graph/dynamic.rs:305-341`
**Action Imperative:** Use batch/UNION pattern.

### C6. get_related Issues 32 Queries Per Call

**Severity:** Performance
**File:** `crates/kavach-surreal/src/graph/dynamic.rs:354-383`
**Action Imperative:** Use single query with IN clause.

### C7. No Indexes on Graph Edge Tables

**Severity:** Performance
**File:** `crates/kavach-surreal/src/schema/*.rs`
**Action Imperative:** Add indexes on edge tables.

### C8. nlm_doc.captured_at Is string Not datetime

**Severity:** Schema
**File:** `crates/kavach-surreal/src/schema/migrations.rs:105`
**Action Imperative:** Change to TYPE datetime.

### C9. Dead "proposal" Category Accepted

**Severity:** Category
**File:** `crates/kavach-rpc/src/methods/recall_integrity.rs:18`
**Action Imperative:** Remove "proposal" from VALID_CATEGORIES.

### C10. KavachBrain Trait Never Implemented

**Severity:** Unused
**File:** `crates/kavach-surreal/src/brain.rs:48`
**Action Imperative:** Implement or remove trait.

### C11. Bridge Query Issues 20 Sequential Queries

**Severity:** Performance
**File:** `crates/kavach-surreal/src/graph/bridges/query/consts.rs:1-121`
**Action Imperative:** Consolidate into single UNION query.

### C12. brain.gap Not Exposed via RPC

**Severity:** Missing
**File:** `crates/kavach-rpc/src/rpc.rs:263-1067`
**Action Imperative:** Register brain.gap endpoint.

---

## D. unwrap/expect in Production Code

### D1. expect("waiter task") on tokio spawn

**Severity:** Medium
**File:** `crates/kavach-rpc/src/state.rs:107`
**Action Imperative:** Replace with graceful error handling.

### D2. unwrap() on lower(raw) in antigravity vendor

**Severity:** Medium
**File:** `crates/kavach-hook/src/vendor/antigravity.rs:114`
**Action Imperative:** Replace with match/if-let.

### D3. unwrap() on lower(raw) in pi vendor

**Severity:** Medium
**File:** `crates/kavach-hook/src/vendor/pi.rs:130`
**Action Imperative:** Replace with match/if-let.

### D4. panic! on unexpected enum variant in kprobe

**Severity:** Medium
**File:** `crates/kavach-chain/src/kprobe.rs:296,305`
**Action Imperative:** Replace with exhaustive match.

### D5. panic! on regex compile failure

**Severity:** Low
**File:** `crates/kavach-patterns/src/destructive_cli_guard/patterns.rs:46`
**Action Imperative:** Use LazyLock with Option<Regex>.

### D6. expect("write temp module")

**Severity:** Low
**File:** `crates/kavach-config/src/modules.rs:82`
**Action Imperative:** Log and continue.

### D7. expect("serialization failed")

**Severity:** Low
**File:** `crates/kavach-rule-storage/src/registry.rs:83,85`
**Action Imperative:** Return Result instead.

---

## E. Error Swallowing Patterns

### E1. Regex .ok() in db_security_guard

**Severity:** High
**File:** `crates/kavach-patterns/src/db_security_guard.rs:16`
**Issue:** Regex compile failure silently skipped.
**Action Imperative:** Log regex compile errors.

### E2. unwrap_or_default() on regex in regex_patterns

**Severity:** High
**File:** `crates/kavach-patterns/src/regex_patterns.rs:30`
**Issue:** Bad regex -> empty pattern -> no detection.
**Action Imperative:** Use LazyLock with Option.

### E3. 6 chained .ok() in observability_guard

**Severity:** Medium
**File:** `crates/kavach-patterns/src/observability_guard.rs:46-51`
**Action Imperative:** Collect regex errors at startup.

### E4. 15 chained .ok() in atomic_ui_guard

**Severity:** Medium
**File:** `crates/kavach-patterns/src/atomic_ui_guard/util.rs:57-71`
**Action Imperative:** Same as E3.

### E5. 20 chained .ok() in database_ops_guard

**Severity:** Medium
**File:** `crates/kavach-patterns/src/database_ops_guard/pattern_set.rs:23-42`
**Action Imperative:** Same as E3.

### E6. unwrap_or_default() in laziness_guard

**Severity:** Low
**File:** `crates/kavach-patterns/src/laziness_guard.rs:238`
**Action Imperative:** Use explicit default handling.

### E7. unwrap_or_default() in skill_manifest

**Severity:** Low
**File:** `crates/kavach-patterns/src/skill_manifest.rs:126`
**Action Imperative:** Use explicit default handling.

---

## F. Concurrency Patterns

### F1. LazyLock<Mutex<Option<HashMap>>> in modules

**Severity:** Medium
**File:** `crates/kavach-config/src/modules.rs:9`
**Issue:** Global mutable state, potential deadlock if held across await.
**Action Imperative:** Use tokio::sync::Mutex for async contexts.

### F2. LazyLock<Mutex<Option<Config>>> in gates_loader

**Severity:** Medium
**File:** `crates/kavach-config/src/gates_loader.rs:11`
**Action Imperative:** Same as F1.

### F3. LazyLock<Mutex<HashMap>> in cache

**Severity:** Medium
**File:** `crates/kavach-config/src/cache.rs:17`
**Action Imperative:** Same as F1.

### F4. Mutex::new(HashMap) in toolbelt cache

**Severity:** Low
**File:** `crates/kavach-engine/src/toolbelt/cache.rs:8`
**Action Imperative:** OK if held briefly.

### F5. Mutex<Vec<Finding>> in hunt/scan

**Severity:** Low
**File:** `crates/kavach-cli/src/cmd/hunt/scan.rs:11`
**Action Imperative:** Correct parallel scan accumulator.

---

## G. Hardcoded Magic Constants

### G1. APPLIED_HEURISTIC_DAYS = 7

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/pre_write_immutable_migration/applied.rs:6`
**Action Imperative:** Make configurable via gate config.

### G2. FLOW_TOP_K = 3

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/session_start/flows.rs:8`
**Action Imperative:** Make configurable.

### G3. CONCEPT_TOP_K = 5

**Severity:** Medium
**File:** `crates/kavach-engine/src/gates/session_start/concepts.rs:8`
**Action Imperative:** Make configurable.

### G4. MAX_REPEATS = 3

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/loop_guard/detect.rs:5`
**Action Imperative:** Make configurable.

### G5. CONCEPT_MARKER_CAP = 8

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/post_write/concept.rs:7`
**Action Imperative:** Make configurable.

### G6. CHARS_PER_TOKEN = 4

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/context_compress.rs:7`
**Action Imperative:** Make configurable.

### G7. PENDING_ADVISORY_CAP = 3

**Severity:** Low
**File:** `crates/kavach-session/src/turn_shadow.rs:54`
**Action Imperative:** Make configurable.

### G8. DECISION_BLOCK_CAP = 4

**Severity:** Low
**File:** `crates/kavach-engine/src/gates/stop_decisions.rs:5`
**Action Imperative:** Make configurable.

---

## H. Lint Suppressions

### H1-H4. 30+ #[expect] attributes

**Severity:** Low
**Files:** `kavach-patterns/src/*.rs`, `kavach-toon/src/lib.rs`
**Issue:** Mostly acceptable for large regex lists and internal DTOs.
**Action Imperative:** Audit annually; prefer self-expiring expect over allow.

---

## I. Hardcoded Paths

### I1. /tmp/{name}.toon in registry_builder

**Severity:** Medium
**File:** `crates/kavach-rule-storage/src/registry_builder.rs:60`
**Action Imperative:** Use tempdir crate.

### I2. /tmp/kavach-agent-output in output_limits

**Severity:** Medium
**File:** `crates/kavach-config/src/output_limits.rs:33`
**Action Imperative:** Use tempdir or env-configurable path.

### I3. ws://127.0.0.1:7710 default endpoint

**Severity:** Low
**File:** `crates/kavach-surreal/src/connection.rs:22`
**Action Imperative:** OK - env override exists.

---

## J. Dead Code / Unused Exports

### J1. apply_agent_memory_schema

**Severity:** Dead
**File:** `crates/kavach-surreal/src/schema_v2.rs`
**Action Imperative:** Delete file and remove export.

### J2. apply_schema_engine

**Severity:** Dead
**File:** `crates/kavach-surreal/src/schema_engine.rs`
**Action Imperative:** Wire into startup or delete.

### J3. KavachBrain trait

**Severity:** Dead
**File:** `crates/kavach-surreal/src/brain.rs:48`
**Action Imperative:** Implement or remove.

---

## Priority Order for Fixes

1. **C1** - Concept edges silently broken (CRITICAL)
2. **C2** - Concept FTS unranked (CRITICAL)
3. **A1/B1** - Score keyword mismatch (HIGH)
4. **E1/E2** - Regex error swallowing (HIGH)
5. **A3** - Budget downgrade suppresses P0Block (MEDIUM)
6. **A4** - RPC failure masquerades as empty queue (MEDIUM)
7. **B2** - Additive budgets waste CPU (MEDIUM)
8. **B4** - Sequential FTS (MEDIUM)
9. **C5/C6** - N+1 query patterns (MEDIUM)
10. **C7** - Missing edge indexes (MEDIUM)
11. **D1-D4** - unwrap/expect in production (MEDIUM)
12. **F1-F3** - LazyLock<Mutex> global state (MEDIUM)
13. **G1-G3** - Hardcoded magic constants (MEDIUM)
14. **I1-I2** - Hardcoded /tmp paths (MEDIUM)
15. **A5-A10, B5-B11, C8-C12, D5-D7, E3-E7, F4-F5, G4-G8, H1-H4, I3, J1-J3** - Low severity, batch together

---

## Action Imperative Format

Each fix should be implemented as a Kavach action imperative:

```
[IMPERATIVE:<gate-name>] <what> -> <do-this> -> retry
```

Example:
```
[IMPERATIVE:pre-write] concept edges broken -> fix relate.rs BEGIN/COMMIT -> retry Write
```

This ensures fixes are discoverable by the harness and can be tracked as roadmap cards.
