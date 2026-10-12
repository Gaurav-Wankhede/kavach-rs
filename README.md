# Kavach (`kavach-rs`)

> **The Sovereign Engine Governor, Deterministic Context Firewall & In-Memory Memory Substrate for Autonomous AI Coding Agents.**
> Stop Context Rot. Eliminate Context Bloat. Cure Context Amnesia. Defeat "Lost in the Middle". Ground Live Truth with Webfind.

[![Crates.io](https://img.shields.io/crates/v/kavach.svg)](https://crates.io/crates/kavach)
[![Documentation](https://docs.rs/kavach/badge.svg)](https://docs.rs/kavach)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/Gaurav-Wankhede/kavach-rs)](https://github.com/Gaurav-Wankhede/kavach-rs/releases)
[![Companion: Webfind](https://img.shields.io/badge/Companion-Webfind%20(Live%20Search)-emerald.svg)](https://github.com/Gaurav-Wankhede/webfind)
[![Rust 1.95+](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)

---

## Table of Contents

- [The Crisis in AI Coding Agents](#the-crisis-in-ai-coding-agents)
  - [1. Context Rot (Drift & Contradiction)](#1-context-rot-drift--contradiction)
  - [2. Context Bloat (Token Inflation & Noise Flooding)](#2-context-bloat-token-inflation--noise-flooding)
  - [3. Context Amnesia (Stateless Session Death)](#3-context-amnesia-stateless-session-death)
  - [4. "Lost in the Middle" (U-Curve Degradation)](#4-lost-in-the-middle-u-curve-attention-degradation)
  - [5. Stale Training Weights & Hallucination (External Grounding Failure)](#5-stale-training-weights--hallucination-external-grounding-failure)
- [What is Kavach?](#what-is-kavach)
- [The Sovereign Tool Ecosystem: Kavach & Webfind](#the-sovereign-tool-ecosystem-kavach--webfind)
- [The 12 Legacy Replacements](#the-12-legacy-replacements)
- [Empirical Telemetry & Benchmarks](#empirical-telemetry--benchmarks)
  - [1,000-Run Execution Benchmark (Hyperfine Statistical Analysis)](#1000-run-execution-benchmark-hyperfine-statistical-analysis)
  - [Token Consumption & Window Compression Matrix](#token-consumption--window-compression-matrix)
- [Core Architecture & Capabilities](#core-architecture--capabilities)
  - [1. Surgical Extraction Engine (`kavach grep`, `kavach glob`)](#1-surgical-extraction-engine)
  - [2. Deterministic Sandboxed Scripting (`kavach script`)](#2-deterministic-sandboxed-scripting)
  - [3. Embedded Turso & SQLite Memory Bank (`kavach db`)](#3-embedded-turso--sqlite-memory-bank)
  - [4. Atomic AST Code Patching (`kavach replace`)](#4-atomic-ast-code-patching)
  - [5. Codebase Lineage & Blast-Radius Tracing (`kavach trace`, `kavach origin`)](#5-codebase-lineage--blast-radius-tracing)
  - [6. Phase-Gated SDLC Engine Governance (`kavach verify`)](#6-phase-gated-sdlc-engine-governance)
- [Installation](#installation)
- [Day-in-the-Life Operational Runbook](#day-in-the-life-operational-runbook)
  - [Phase 1: Discovery & Live Grounding (Kavach + Webfind)](#phase-1-discovery--live-grounding-kavach--webfind)
  - [Phase 2: Surgical State Inspection & Caller Tracing](#phase-2-surgical-state-inspection--caller-tracing)
  - [Phase 3: Deterministic Scripting & Atomic Mutation](#phase-3-deterministic-scripting--atomic-mutation)
  - [Phase 4: SDLC Verification & Cognitive Memory Checkpoint](#phase-4-sdlc-verification--cognitive-memory-checkpoint)
- [Canonical Multi-Agent Governance (`prompts/`)](#canonical-multi-agent-governance-prompts)
- [Configuration Reference](#configuration-reference)
- [Author & Maintenance](#author--maintenance)

---

## The Crisis in AI Coding Agents

Modern Large Language Model (LLM) coding agents—Claude Code, Gemini CLI, Cursor, Windsurf, Devin, and Aider—suffer from systemic architectural failures caused by relying on legacy POSIX CLI tools (`cat`, `find`, `grep`, `sed`, `node -e`, `python -c`) and ungrounded pre-trained weights.

Kavach acts as a deterministic firewall between the LLM and the operating system, eliminating these pathologies at the syscall level.

```
+-------------------------------------------------------------------------+
|                         LLM AGENT CONTEXT WINDOW                        |
|                                                                         |
|   +-------------------+                     +-----------------------+   |
|   | 1. CONTEXT ROT    |                     | 2. CONTEXT BLOAT      |   |
|   | Drift across turns|                     | 40k tokens from `cat` |   |
|   +---------+---------+                     +-----------+-----------+   |
|             |                                           |               |
|             v                                           v               |
|   ===================================================================   |
|   |                  KAVACH DETERMINISTIC FIREWALL                  |   |
|   ===================================================================   |
|             ^                         ^                 ^               |
|             |                         |                 |               |
|   +---------+---------+     +---------+---------+     +-+-------------+ |
|   | 3. CONTEXT AMNESIA|     | 4. LOST IN MIDDLE |     | 5. STALE WEIGHTS|
|   | Wipe between runs |     | U-curve drop      |     | Hallucinations| |
|   +-------------------+     +-------------------+     +---------------+ |
+-------------------------------------------------------------------------+
```

### 1. Context Rot (Drift & Contradiction)
- **The Failure:** As agents execute across multiple turns, context accumulates stale directory listings, outdated file versions, and dead diffs. The model begins hallucinating nonexistent APIs, using deleted imports, and contradicting its own architectural decisions.
- **The Kavach Solution:** `kavach working-state` and `kavach origin` maintain deterministic, point-in-time ground truth. Instead of re-reading mutated files, Kavach validates file hashes and origin states on every mutation.

### 2. Context Bloat (Token Inflation & Noise Flooding)
- **The Failure:** When an agent inspects a 2,000-line file or runs `cat package.json` / `find .`, it dumps thousands of tokens into the context window. Within 5-10 turns, the context reaches 100k+ tokens, skyrocketing API costs and causing the agent to slow down exponentially.
- **The Kavach Solution:** Kavach permanently enforces **Zero Whole-File Reads**. Tools like `kavach grep` (AST-aware pattern extraction), `kavach glob` (bounded search), and structured field projections extract only the exact AST slice or JSON key needed—reducing token consumption by up to **99.28%**.

### 3. Context Amnesia (Stateless Session Death)
- **The Failure:** When a shell session ends or the agent hits context compaction, all intermediate reasoning, verified architecture decisions, test states, and task roadmaps evaporate. The agent starts from scratch on the next turn.
- **The Kavach Solution:** An embedded **Turso / libSQL SQLite engine** (`kavach db`) with vector embeddings and typed schema memory stores kanban cards, architectural decisions, and session states locally on disk.

### 4. "Lost in the Middle" (U-Curve Attention Degradation)
- **The Failure:** Empirical research (*Liu et al., Stanford/Berkeley*) proves that LLM retrieval accuracy degrades severely when relevant instructions or facts are buried in the middle of long contexts (the U-shaped attention curve).
- **The Kavach Solution:** Kavach's surgical context slices place exact facts directly adjacent to the immediate prompt turn, completely bypassing the degraded middle of the attention window.

### 5. Stale Training Weights & Hallucination (External Grounding Failure)
- **The Failure:** LLM agents inherently default to their frozen pre-trained weights for external library versions, API signatures, CLI flags, security CVEs, and breaking changes. Training weights are inherently stale from day one. When an agent guesses an API based on frozen memory, code generation fails or introduces subtle runtime bugs.
- **The Kavach Solution:** Kavach strictly mandates the **Tabula Rasa Law** during discovery: pre-trained memory is never admissible evidence for external facts. External APIs and technical claims must be grounded live via **[Webfind](https://github.com/Gaurav-Wankhede/webfind)** with explicit **Time Awareness** (`TODAY=$(date -u +%Y-%m-%d)`).

---

## The Sovereign Tool Ecosystem: Kavach & Webfind

Autonomous agents operate under strict non-overlapping domain sovereignty:

```
+---------------------------------------------------------------------------+
|                          AUTONOMOUS AGENT HARNESS                         |
|                                                                           |
|   +-----------------------------------+   +---------------------------+   |
|   |         KAVACH (`kavach-rs`)      |   |   WEBFIND (`webfind`)     |   |
|   |     LOCAL CODEBASE SOVEREIGNTY    |   |  LIVE INTERNET GROUNDING  |   |
|   +-----------------+-----------------+   +-------------+-------------+   |
|                     |                                   |                 |
|                     v                                   v                 |
|   * Zero whole-file reads                 * Live search engine (zero cost)|
|   * AST grep & atomic replace             * Tabula Rasa external grounding|
|   * Embedded Turso memory substrate       * Time Awareness date guards    |
|   * SDLC phase-gate enforcement           * Compact markdown web fetch    |
|   * Blast-radius symbol tracing           * Knowledge graph indexing      |
+---------------------------------------------------------------------------+
```

- **[Kavach (`kavach-rs`)](https://github.com/Gaurav-Wankhede/kavach-rs):** Governs local codebase state, AST mutations, context firewalls, and persistent session memory.
- **[Webfind (`webfind`)](https://github.com/Gaurav-Wankhede/webfind):** Governs external web search, live documentation grounding, and URL extraction. It guarantees agents never rely on frozen training weights.

---

## The 12 Legacy Replacements

Kavach replaces the brittle, bloated POSIX and runtime tools that cause AI agent degradation:

| Legacy Tool | Failure Mode in AI Agents | Allowed Kavach / Sovereign Command | Advantage |
| :--- | :--- | :--- | :--- |
| **`cat` / `bat`** | Dumps entire files, burning 5k–30k tokens per read. | `kavach grep` / slice projections | Zero whole-file reads; sub-slice AST extraction only. |
| **`jq`** | Brittle syntax; shell quoting errors; unindexed scans. | `jaq` / `dasel` / `kavach script` | Rust-native, schema-validated projection. |
| **`grep` / `rg`** | Unbounded text hits with noisy context lines. | `kavach grep "<pattern>" <path> -C 2` | AST-aware code pattern matching with bounded context. |
| **`find` / `fd`** | Unbounded directory walks pollute context. | `kavach glob "<pattern>" <path> --max-depth 4` | Strict depth limits, deterministic sorted output. |
| **`curl` / `wget`** | Unformatted HTML blobs pollute context with noise. | `webfind fetch --compact "<url>"` | Clean semantic markdown extraction for agents. |
| **`node -e`** | 118 ms startup latency; V8 heap overhead; Node dependency. | `kavach script -e "<expr>"` | **4.7 ms** instant Rhai AST evaluation; zero dependencies. |
| **`python3 -c`** | 57 ms cold-start; environment/version drift; GIL overhead. | `kavach script -e "<expr>"` | **12x faster**; deterministic sandbox; no virtualenvs. |
| **`bun -e`** | 7.4 ms startup; unvetted external runtime in production environments. | `kavach script -e "<expr>"` | Pure Rust in-process runtime with memory safety. |
| **`sed` / `awk`** | Destructive regex failures, macOS vs GNU dialect bugs. | `kavach replace '<old>' '<new>' <file>` | Atomic Aho-Corasick replacement with automatic backup. |
| **`sqlite3`** | External process spawn; no vector search; brittle schemas. | `kavach db query "<sql>"` | Embedded Turso/libSQL engine with schema migrations. |
| **`git log`** | Dumps unbounded commit histories and diff noise. | `kavach trace "<symbol>" <path>` | Symbol-level caller network and semantic blast radius. |
| **`make` / `bash`** | Non-deterministic scripts, untracked SDLC phase gates. | `kavach verify --phase <gate>` | Deterministic, audited phase gate validation. |

---

## Empirical Telemetry & Benchmarks

All benchmarks were collected on an Apple Silicon M-Series workstation running macOS Darwin, measured using `hyperfine` with **1,000 statistical sample runs**, warm cache, and release-optimized binaries.

### 1,000-Run Execution Benchmark (Hyperfine Statistical Analysis)

Evaluating inline AST/expression evaluation engines across 1,000 executions:

```
Benchmark 1: kavach script -e "let x = 10; x * 2"  (1,000 runs)
  Time (mean ± σ):      4.7 ms ±   0.4 ms    [User: 2.1 ms, System: 2.2 ms]
  Range (min … max):    3.7 ms …   7.5 ms    1000 runs

Benchmark 2: bun -e "const x = 10; console.log(x * 2)"  (1,000 runs)
  Time (mean ± σ):      7.4 ms ±   0.6 ms    [User: 4.8 ms, System: 2.3 ms]
  Range (min … max):    6.1 ms …  11.2 ms    1000 runs

Benchmark 3: python3 -c "x = 10; print(x * 2)"  (1,000 runs)
  Time (mean ± σ):     57.6 ms ±   1.8 ms    [User: 41.2 ms, System: 14.8 ms]
  Range (min … max):   52.4 ms …  66.1 ms    1000 runs

Benchmark 4: node -e "const x = 10; console.log(x * 2)"  (1,000 runs)
  Time (mean ± σ):    118.7 ms ±   3.2 ms    [User: 92.1 ms, System: 24.3 ms]
  Range (min … max):  111.0 ms … 131.4 ms    1000 runs
```

| Engine | Mean Latency (1,000 runs) | Relative Speedup vs Node.js | Memory Overhead |
| :--- | :--- | :--- | :--- |
| **`kavach script` (Rhai AST)** | **4.7 ms ± 0.4 ms** | **25.2x faster** | **~2.1 MB** (Embedded) |
| `bun -e` | 7.4 ms ± 0.6 ms | 16.0x faster | ~32 MB (JIT Runtime) |
| `python3 -c` | 57.6 ms ± 1.8 ms | 2.1x faster | ~18 MB (CPython VM) |
| `node -e` | 118.7 ms ± 3.2 ms | 1.0x (Baseline) | ~45 MB (V8 Runtime) |

### Token Consumption & Window Compression Matrix

Comparing standard developer actions performed via legacy POSIX tools vs Kavach surgical extraction:

| Action / Query | Legacy Command | Legacy Tokens | Kavach Command | Kavach Tokens | Context Window Savings |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Inspect single JSON key** | `cat AGENTS.json` (42 KB) | 9,299 tokens | `jaq -r '.witnesses' AGENTS.json` | 66 tokens | **99.28% reduction** |
| **Locate Rust function signature** | `cat src/main.rs` (38 KB) | 8,450 tokens | `kavach grep "fn main" src/ -C 2` | 42 tokens | **99.50% reduction** |
| **Find symbol caller network** | `grep -rn "execute_task" .` | 1,820 tokens | `kavach trace "execute_task" .` | 58 tokens | **96.81% reduction** |
| **Evaluate math / array logic** | `python3 -c "print([...])"` | 185 tokens | `kavach script -e "[...].len()"` | 12 tokens | **93.51% reduction** |
| **Read project task queue** | `cat ROADMAP.md` (full file) | 3,400 tokens | `kavach db kanban --todo` | 110 tokens | **96.76% reduction** |

---

## Core Architecture & Capabilities

```
+-------------------------------------------------------------------------+
|                           KAVACH-RS ARCHITECTURE                        |
|                                                                         |
|  +--------------------+  +--------------------+  +-------------------+  |
|  |   kavach grep      |  |   kavach script    |  |     kavach db     |  |
|  | AST / regex filter |  | Rhai in-memory VM  |  |  Turso / libSQL   |  |
|  +---------+----------+  +---------+----------+  +---------+---------+  |
|            |                       |                       |            |
|            +-----------------------+-----------------------+            |
|                                    |                                    |
|                                    v                                    |
|                      [ DETERMINISTIC DISPATCH ENGINE ]                  |
|                                    |                                    |
|            +-----------------------+-----------------------+            |
|            |                       |                       |            |
|  +---------+----------+  +---------+----------+  +---------+---------+  |
|  |   kavach replace   |  |    kavach trace    |  |   kavach verify   |  |
|  | Aho-Corasick atomic|  | Symbol blast-radius|  |  SDLC phase gates |  |
|  +--------------------+  +--------------------+  +-------------------+  |
+-------------------------------------------------------------------------+
```

### 1. Surgical Extraction Engine
- **Command:** `kavach grep "<pattern>" <path> [-C <lines>]`
- **Design:** Zero whole-file dumps. Searches polyglot source trees using AST structural matching or regular expressions, returning bounded context slices directly consumable by LLM prompts.
- **Token Impact:** Drops average search context cost from 2,000+ tokens to <50 tokens.

### 2. Deterministic Sandboxed Scripting
- **Command:** `kavach script -e "<rhai-expression>"`
- **Design:** Powered by the embedded [Rhai](https://rhai.rs) engine. Executes in **4.7 ms** with pure memory isolation. Eliminates external interpreter dependencies (`node`, `bun`, `python`) and prevents sub-process zombie spawning.

### 3. Embedded Turso & SQLite Memory Bank
- **Command:** `kavach db [query|kanban|memory]`
- **Design:** Local SQLite / Turso storage engine backing agent cognitive memory.
- **Capabilities:**
  - `kavach db kanban`: Persistent roadmap and task card tracking.
  - `kavach db memory`: Key-value cognitive store storing verified architectural decisions across session boundaries.
  - `kavach working-state`: Checkpoints active memory state so agents never experience context amnesia between restarts.

### 4. Atomic AST Code Patching
- **Command:** `kavach replace '<target_string>' '<replacement_string>' <file>`
- **Design:** Uses `aho_corasick` search and atomic temporary files. Guarantees that files are never partially written or corrupted during LLM generation aborts.
- **Safety:** Automatically creates safety backups and verifies file integrity before atomic rename.

### 5. Codebase Lineage & Blast-Radius Tracing
- **Command:** `kavach trace "<symbol>" <path> [--callers]`
- **Design:** Analyzes AST call graphs and inbound/outbound symbol references. Provides the AI agent with the exact blast radius before a refactor occurs, preventing unintended regressions.

### 6. Phase-Gated SDLC Engine Governance
- **Command:** `kavach verify [--phase <phase_name>]`
- **Design:** Enforces strict software development lifecycle (SDLC) quality gates. Blocks agents from marking tasks complete until static analysis, test suites, and schema validation passes with 0 errors.

---

## Installation

### From Source (Cargo)

Ensure you have Rust 1.95+ installed:

```bash
# Clone the canonical repository
git clone https://github.com/Gaurav-Wankhede/kavach-rs.git
cd kavach-rs

# Compile with maximum release optimizations
cargo build --release

# Install binary to system PATH
cp target/release/kavach /usr/local/bin/kavach
```

### Companion CLI: Webfind

Install [Webfind](https://github.com/Gaurav-Wankhede/webfind) for live, zero-cost external research grounding:

```bash
git clone https://github.com/Gaurav-Wankhede/webfind.git
cd webfind
cargo build --release
cp target/release/webfind /usr/local/bin/webfind
```

### Verification

```bash
kavach --version
kavach script -e "print('Kavach Sovereign Engine Online');"
webfind --version
```

---

## Day-in-the-Life Operational Runbook

Here is the exact lifecycle of how an autonomous AI coding agent interacts with a project governed by Kavach and grounded by [Webfind](https://github.com/Gaurav-Wankhede/webfind):

```
[ PHASE 1: DISCOVERY & LIVE GROUNDING ]
       |
       +---> kavach glob "*.rs" src/ --max-depth 3               (Local Codebase Discovery)
       +---> webfind search --live "<API/Crate> $(date +%Y)"       (Live External Grounding)
       +---> jaq -r '.subsystems' config.json                    (Surgical Schema Projection)
       |
[ PHASE 2: SURGICAL INSPECTION & BLAST RADIUS ]
       |
       +---> kavach grep "pub fn dispatch" src/ -C 2
       +---> kavach trace "dispatch" src/ --callers
       |
[ PHASE 3: EXECUTION & ATOMIC MUTATION ]
       |
       +---> kavach script -e "compute_weights([1, 2, 3])"
       +---> kavach replace 'let timeout = 5;' 'let timeout = 10;' src/config.rs
       |
[ PHASE 4: VERIFICATION & COGNITIVE MEMORY ]
       |
       +---> kavach verify --phase unit-tests
       +---> kavach db memory --record "Set dispatch timeout to 10s"
```

### Phase 1: Discovery & Live Grounding (Kavach + Webfind)

#### 1. Local Codebase Discovery (Zero Guessing)
Before touching any code, the agent discovers project structure without probing blindly:
```bash
kavach glob "*.rs" src/ --max-depth 3
```

#### 2. External Live Grounding with Time Awareness ([Webfind](https://github.com/Gaurav-Wankhede/webfind))
> **The Tabula Rasa Law:** Pre-trained weights are **NEVER** admissible as evidence for external APIs, crate versions, CLI flags, or platform behaviors.
> Every factual claim must be witnessed live via **[Webfind](https://github.com/Gaurav-Wankhede/webfind)** in the exact turn it is made.

The agent enforces **Time Awareness** using the system date guard:
```bash
# Anchor search to the current year to reject stale documentation
CURRENT_YEAR=$(date -u +%Y)
webfind search --live --depth deep --hybrid --limit 6 -o json "axum 0.8 routing $CURRENT_YEAR"
```

To inspect official documentation without dumping unformatted HTML:
```bash
webfind fetch --compact "https://docs.rs/axum/latest/axum/"
```

### Phase 2: Surgical State Inspection & Caller Tracing

The agent inspects configuration and caller networks surgically without whole-file reads:
```bash
# Surgical JSON inspection
jaq -r '.universal_behaviour.imperatives[] | .id' prompts/AGENTS.json

# AST caller network & blast-radius analysis before mutating symbols
kavach trace "handle_request" src/ --callers
```

### Phase 3: Deterministic Scripting & Atomic Mutation

Calculations and data transformations execute inside the in-process Rhai VM in **4.7 ms**, followed by atomic patching:
```bash
# In-process AST logic (replaces python3 -c and node -e)
kavach script -e "let arr = [10, 20, 30]; arr.map(|x| x * 2)"

# Atomic Aho-Corasick replacement (zero file destruction)
kavach replace 'let max_retries = 3;' 'let max_retries = 5;' src/engine.rs
```

### Phase 4: SDLC Verification & Cognitive Memory Checkpoint

The agent enforces the SDLC phase gate and commits decisions to the persistent memory bank:
```bash
# Phase verification gate
kavach verify --phase pre-commit

# Record decision to embedded Turso database to cure context amnesia
kavach db memory --set "engine.max_retries=5" --reason "Mitigate API rate limit transients"
```

---

## Canonical Multi-Agent Governance (`prompts/`)

Kavach includes battle-tested prompt specifications designed for production agent harnesses in the [`prompts/`](prompts/) directory:

- [`prompts/AGENTS.json`](prompts/AGENTS.json): **Single Source of Truth (SSOT).** Schema-validated machine configuration declaring toolbelt mappings, domain witnesses, universal imperatives, and allowlists.
- [`prompts/AGENTS.md`](prompts/AGENTS.md): **Human-Readable Action Imperative Index.** Markdown interface pointing directly into `AGENTS.json` via `jaq` projections.
- [`prompts/CLAUDE.md`](prompts/CLAUDE.md): Dedicated harness instruction file for Claude Code and Anthropic agent runners.
- [`prompts/SOUL.md`](prompts/SOUL.md): Core agent persona, immutable safety boundaries, and operational directives.

---

## Configuration Reference

Kavach stores project-level memory and telemetry in `.kavach/` within the repository root:

```json
{
  "governance": {
    "zero_whole_file_reads": true,
    "banned_interpreters": ["node -e", "python -c", "bun -e"],
    "max_context_lines": 100,
    "tabula_rasa_grounding": {
      "enforced": true,
      "provider": "webfind",
      "live_flag": "--live"
    }
  },
  "memory": {
    "engine": "turso_embedded",
    "db_path": ".kavach/kavach.db"
  },
  "phases": {
    "lint": "cargo clippy -- -D warnings",
    "test": "cargo test",
    "verify": "kavach verify --phase strict"
  }
}
```

---

## Author & Maintenance

- **Author & Architect:** [Gaurav Wankhede](https://github.com/Gaurav-Wankhede) (`gauravanilwankhede2002@gmail.com`)
- **Repositories:**
  - Kavach Engine: [`Gaurav-Wankhede/kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)
  - Live Search Grounding: [`Gaurav-Wankhede/webfind`](https://github.com/Gaurav-Wankhede/webfind)
- **License:** MIT License

*Designed and engineered for zero context rot, zero token waste, live web grounding, and deterministic AI agent governance.*
