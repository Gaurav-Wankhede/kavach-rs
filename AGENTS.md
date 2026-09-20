# AGENTS.md — Global Agent Architecture & Action Imperative Harness Directives

## 1. Core Operating Principles (System Prompt as Action Imperative Router Only)
- **Zero Bloat & Metadata Router ONLY:** The global prompt holds metadata indexes, fail-closed boundaries, and operational action imperatives only. It never carries fat documentation, duplicate templates, or monolithic instructions that cause context rot.
- **Autonomous Skill & Plugin Activation (Zero-Permission Mandate):** Autonomously route and execute specialized skills/plugins based on intent metadata. Never wait for user permission to invoke tools.
- **Zero-Assumption Mandate:** Never assume file layout, dependencies, or tool availability. Read, execute, and verify live evidence directly. Never ask the user to run commands you can execute.
- **Gate Verdicts as Action Imperatives:** Gate denials are automatic redirects providing the next immediate action, never a stopping point.
- **3-Witness Verification:** Completion requires physical artifact existence (`fd`), clean atomic diff (`rtk git diff --stat`), and strict compiler/test pass (`rtk cargo clippy --workspace -- -D warnings && rtk cargo nextest run`).

---

## 2. Governed 4-Stage Architecture & Reasoning Paradigms
- **Stage 1 (Perimeter Layer - Fail-Closed Governance & SDD):**
  - All tool invocations are intercepted. P0 deny on legacy POSIX inspection tools (`cat`, `head`, `tail`).
  - Spec-Driven Development (SDD): Reject tasks lacking an immutable contract or roadmap card (`kavach db write --category roadmap`).
  - Domain-Driven Design (DDD): Enforce crate-level bounded contexts (`kavach-types`, `kavach-rpc`); zero model leakage.
- **Stage 2 (Intent Router & YAGNI Filter):**
  - **YAGNI Ladder:** (1) Need now? (2) Reuse codebase helper? (3) Standard library first? (4) Native dependency? (5) Single line before fifty.
  - **Fast Direct Path ($O(1)$):** Deterministic queries (`kavach origin <SYM>`, `rtk rg -n`, `jaq`, `tokei`) bypass intermediate reasoning loops.
- **Stage 3A (Multi-Step Engine - TDD & 4-Tier AST Slicing):**
  - **TDD Red-Green-Refactor:** Enforce failing test before production code edit (`tdd_guard`).
  - **4-Tier Hybrid AST Protocol (Strict Toolbelt Law):**
    1. *Tier 1 (Topology):* `rtk tree -L <depth>` or `erd -L <depth>`. Zero content dumps.
    2. *Tier 2 (Hybrid Coordinates):* Fast regex line sweep (`rtk rg -n '<pattern>'`) paired with Concrete Syntax Tree boundary extraction (`rtk run -- sg run -p '<pattern>' --json=compact`).
    3. *Tier 3 (Surgical Slicing):* Read strictly identified line ranges (`bat -p -r <start>:<end> <file>`). Blind whole-file reads, arbitrary chunk paging, and blind `head`/`tail` slicing are strictly banned.
    4. *Tier 4 (In-Place Edit):* Single-file mutation in place via `replace_file_content` or `sd`. Zero sibling/duplicate sprawl (`_v2`, `_new`).
- **Stage 3B (Tree of Thought Engine - DOD & Branching RCA):**
  - For complex bugs and borrow-checker conflicts: instantiate parallel hypothesis branches ($O(B^D)$), evaluate evidence, and prune dead branches via compiler diagnostics and negative memory (`kavach think`, `kavach mistake list`).
  - Data-Oriented Design (DOD): Struct of Arrays (SoA), struct field alignment ($8 \to 4 \to 2 \to 1$), Small String Optimization (`CompactStr`), 8-byte byte offsets (`TextRange`), and arena allocations (`bumpalo`).
- **Stage 4 (Perimeter Verification - DoD & EDD Exit Gate):**
  - Definition of Done: Verify the 3 physical witnesses.
  - Evaluation-Driven Development (EDD): Continuous benchmark/trace validation (`kavach db ope-evaluate`, `rtk gain`).

---

## 3. Toolbelt Invariant & RTK Proxy Mandate (Canonical 21 Tools)
- **RTK-First Proxy:** All terminal commands must be routed through `rtk` (`rtk cargo`, `rtk rg`, `rtk find`, `rtk ls`, `rtk tree`, `rtk diff`, `rtk git`) to strip 60%–80% terminal noise and ANSI escapes.
- **Banned Legacy Tools:** `cat`, `head`, `tail`, `grep`, `sed`, `find`, `jq`, `du`, `ps`, `curl` are replaced by the Rust toolbelt:
  - Token Proxy $\to$ `rtk`
  - Text Regex $\to$ `rg` / `rtk rg -n`
  - AST Matcher $\to$ `sg` / `rtk run -- sg`
  - Bounded Slicing $\to$ `bat -p -r <start>:<end>`
  - File Find $\to$ `fd` / `rtk find`
  - Directory Tree $\to$ `erd` / `rtk tree`
  - In-Place Mutation $\to$ `sd`
  - Diff Engine $\to$ `difft` / `rtk diff`
  - JSON / Structured Data $\to$ `jaq` (filter) / `gron` (flatten)
  - Metrics $\to$ `tokei` / `rtk wc`
  - HTTP $\to$ `xh`
  - Process / Disk $\to$ `procs` / `dust`
  - Command Runner $\to$ `just`
  - Benchmarks $\to$ `hyperfine`

---

## 4. Property-Based Testing Mandate (`proptest` + `rtk cargo nextest`)
- **Ban on Hallucinated Single-Case Mocks:** Single-case hardcoded assertions (`assert_eq!(f(1), 2)`) are forbidden for verifying core algorithms and invariants.
- **Property-Based Fuzzing:** Core algorithms must use `proptest!` blocks testing symmetry (serde roundtrips), saturation boundaries, and invariants across thousands of generated inputs.
- **Nextest Runner:** All tests must run through `rtk cargo nextest run` for fast, isolated execution.

---

## 5. Metadata-Driven Database Querying (Zero-Dumping Law)
- **Never dump entire tables or broad result sets into LLM context.**
- **Tiered Retrieval Protocol:**
  - *Tier 1 (Keys & Metadata):* `kavach db query --project <slug> --category <cat> --field key` (Zero body tokens).
  - *Tier 2 (Bounded Horizon):* `kavach db query ... --depth <chars>` or `kavach db search --status <status> --since <dur> --limit <N>`.
  - *Tier 3 (Surgical Entity):* `kavach db get --key <key> --full` piped to `jaq` for projection.
- **Token Compression:** Use `--compact` flag on high-volume queries.

---

## 6. Mechanical Sympathy & Zero-Heap Memory Directives
- **Small String Optimization (SSO):** Standardize identifiers on `CompactStr` ($\le 24$ bytes inline on stack).
- **Zero-Heap Spans (`TextRange`):** Store 8-byte byte-offsets (`start: u32`, `end: u32`) rather than 32-byte 2D line/col coordinates. Defer human line/column formatting to terminal boundary.
- **Cache-Line Struct Packing ($8 \to 4 \to 2 \to 1$):** Order struct fields from largest alignment to smallest to eliminate compiler padding holes (saving up to 40% memory).
- **Lock-Striped Sharding:** Replace single global `Mutex<HashMap>` with hash-indexed striped shards (`Box<[RwLock<Shard<V>>]>`).
- **Crash-Safe Persistence:** Local SQLite / libSQL with WAL pragma triad (`wal`, `synchronous = normal`, `busy_timeout = 5000`); zero external database daemons for local-first operations.
