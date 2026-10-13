<p align="center">
  <img src="docs/assets/banner.png" alt="Kavach Banner" width="100%" onerror="this.style.display='none'">
</p>

# Kavach (`kavach-rs`)

<p align="center">
  <b>Stop burning $15/day feeding whole files and bloated MCP schemas to AI agents.</b><br>
  The Sovereign Engine Governor, Deterministic Context Firewall & Persistent Memory Substrate for Autonomous AI Coding Agents.
</p>

<p align="center">
  <a href="https://crates.io/crates/kavach"><img src="https://img.shields.io/crates/v/kavach.svg" alt="Crates.io"></a>
  <a href="https://docs.rs/kavach"><img src="https://docs.rs/kavach/badge.svg" alt="Documentation"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://github.com/Gaurav-Wankhede/kavach-rs/releases"><img src="https://img.shields.io/github/v/release/Gaurav-Wankhede/kavach-rs" alt="Release"></a>
  <a href="https://github.com/Gaurav-Wankhede/webfind"><img src="https://img.shields.io/badge/Companion-Webfind%20(Live%20Search)-emerald.svg" alt="Webfind Companion"></a>
  <img src="https://img.shields.io/badge/rust-1.95%2B-orange.svg" alt="Rust 1.95+">
  <img src="https://img.shields.io/badge/MCP%20Schema%20Tax-0%20Tokens-brightgreen.svg" alt="Zero MCP Schema Tax">
</p>

---

## The Crisis in AI Coding Agents (Why Your Bills Explode)

When coding agents (Claude Code, Cursor, Windsurf, Gemini CLI, Devin) operate in your codebase, two compounding silent killers drain your token budget and degrade reasoning:

1. **The MCP Schema Tax (+8,000 to 20,000 tokens/turn):** Model Context Protocol (MCP) servers inject massive JSON-Schema definitions into the agent's system prompt on **every single turn** before the user even types a character.
2. **Whole-File Dumps (`cat src/lib.rs`):** Agents read entire 2,000-line files to inspect single functions, quickly consuming 100k+ tokens, slowing down responses, and causing catastrophic **Context Rot** and **"Lost in the Middle"** retrieval failure.

**Kavach** (`kavach-rs`) is a single, blazing-fast static Rust binary that sits between the agent and your machine. It enforces **Zero Whole-File Reads** and **Zero Schema Tax**:

```
+---------------------------------------------------------------------------------------------------+
|                                     AGENT CONTEXT WINDOW TURNS                                    |
|                                                                                                   |
|   [ MCP Approach ]       [ 15k Tokens JSON-Schema Overhead ] + [ 50k Tokens Raw File Dump ]       |
|                          ===================================================================>     |
|                          Result: 65,000+ tokens burned per turn, attention diluted, high latency  |
|                                                                                                   |
|   [ Kavach Approach ]    [ 0 Tokens Schema Tax ] + [ 42 Tokens Exact AST Function Slice ]         |
|                          ===================================================================>     |
|                          Result: 42 tokens, sub-millisecond AST extraction, zero amnesia          |
+---------------------------------------------------------------------------------------------------+
```

---

## Telemetry: Kavach vs MCP vs Legacy Tools

Measured on an Apple Silicon M-Series workstation running release-optimized binaries:

| Scenario / Operation | Legacy Tool (`cat`, `node`, `grep`) | Standard MCP Server | **Kavach (`kavach-rs`)** | Win Factor |
| :--- | :--- | :--- | :--- | :--- |
| **System Prompt Tax** | 0 tokens | **8,000 – 18,000 tokens / turn** | **0 tokens / turn** | **100% Tax Eliminated** |
| **Inspect Rust Function** | `cat src/main.rs` (8,450 tokens) | ~8,450 tokens (raw file payload) | `kavach grep "fn main" src/ -C 2` (**42 tokens**) | **99.50% Savings** |
| **Inspect JSON Configuration** | `cat AGENTS.json` (9,299 tokens) | 9,299 tokens | `kavach json -j .witnesses AGENTS.json` (**66 tokens**) | **99.28% Savings** |
| **Caller Network Analysis** | `grep -rn` (1,820 tokens) | Variable / Unstructured | `kavach trace "dispatch" .` (**58 tokens**) | **96.81% Savings** |
| **Inline Scripting Latency** | `node -e` (118.7 ms) | IPC Roundtrip (~40-80 ms) | `kavach script -e` (**4.7 ms**, Rhai AST) | **25.2× Faster** |
| **State Memory Across Turns** | None (Context Amnesia) | Ephemeral / Non-relational | **Embedded Turso / libSQL SQLite** | **Zero Amnesia** |

---

## 60-Second Quickstart

### 1. Install Kavach
```bash
# Install directly from source or via cargo
cargo install --path crates/kavach-cli --locked
cargo clean
```

### 2. Register Your Project & Initialize Memory
```bash
kavach db register --slug my-app --path $(pwd)
```

### 3. Surgical Code Extraction (Extract Only What You Need)
```bash
# Search AST functions with zero file dumps
kavach grep "pub fn dispatch" src/

# Trace symbol caller networks and blast radius
kavach trace "dispatch" src/ --callers
```

### 4. Zero-Cost Live Web Grounding ([Webfind](https://github.com/Gaurav-Wankhede/webfind))
```bash
# Ground external documentation live without stale training hallucinations
webfind search --live "axum 0.8 routing $(date +%Y)"
```

---

## Table of Contents

- [The 5 Pathologies of AI Coding Agents](#the-5-pathologies-of-ai-coding-agents)
- [The Sovereign Tool Ecosystem: Kavach & Webfind](#the-sovereign-tool-ecosystem-kavach--webfind)
- [The 12 Legacy Replacements](#the-12-legacy-replacements)
- [Empirical Telemetry & Benchmarks](#empirical-telemetry--benchmarks)
  - [1,000-Run Execution Benchmark (Hyperfine Statistical Analysis)](#1000-run-execution-benchmark-hyperfine-statistical-analysis)
  - [Token Consumption & Window Compression Matrix](#token-consumption--window-compression-matrix)
- [Core Architecture & Capabilities](#core-architecture--capabilities)
- [Installation](#installation)
- [Day-in-the-Life Operational Runbook](#day-in-the-life-operational-runbook)
- [Canonical Multi-Agent Governance (`prompts/`)](#canonical-multi-agent-governance-prompts)
- [Configuration Reference](#configuration-reference)
- [Author & Maintenance](#author--maintenance)

---

## The 5 Pathologies of AI Coding Agents

Modern Large Language Model (LLM) coding agents—Claude Code, Gemini CLI, Cursor, Windsurf, Devin, and Aider—suffer from systemic architectural failures caused by relying on legacy POSIX CLI tools (`cat`, `find`, `grep`, `sed`, `node -e`, `python -c`), ungrounded pre-trained weights, and heavy MCP schemas.

Kavach acts as a deterministic firewall between the LLM and the operating system, eliminating these pathologies at the syscall level.

### 1. Context Rot (Drift & Contradiction)
- **The Failure:** As agents execute across multiple turns, context accumulates stale directory listings, outdated file versions, and dead diffs. The model begins hallucinating nonexistent APIs, using deleted imports, and contradicting its own architectural decisions.
- **The Kavach Solution:** `kavach working-state` and `kavach origin` maintain deterministic, point-in-time ground truth. Instead of re-reading mutated files, Kavach validates file hashes and origin states on every mutation.

### 2. Context Bloat & The Hidden "MCP Schema Tax"
- **The Failure (File Dumps & MCP Overhead):** When an agent inspects a 2,000-line file with `cat` or connects multiple Model Context Protocol (MCP) servers, token overhead explodes. MCP servers inject 8,000 to 20,000+ tokens of heavy JSON Schema definitions into the system prompt on **every single turn** before the user even types, diluting model routing attention. Then, whole-file tools dump entire source trees, causing 100k+ token saturation in 5 turns.
- **The Kavach Solution (Zero-Schema Overhead + AST Slicing):** Kavach permanently enforces **Zero Whole-File Reads & Zero-Schema Overhead**. As a native Rust static binary CLI, Kavach incurs **0 system-prompt schema tokens**. Tools like `kavach grep` (AST-aware pattern extraction), `kavach glob` (bounded search), and structured field projections extract only the exact AST slice needed—reducing active context consumption by up to **99.50%**.

### 3. Context Amnesia (Stateless Session Death)
- **The Failure:** When a shell session ends or the agent hits context compaction, all intermediate reasoning, verified architecture decisions, test states, and task roadmaps evaporate. The agent starts from scratch on the next turn.
- **The Kavach Solution:** An embedded **Turso / libSQL SQLite engine** (`kavach db`) with a complete **Dependency Graph Database**, **Roadmap DAG Scheduler**, **Mistakes Ledger**, and **Ontology Knowledge Graph** preserves state across all sessions.

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
|   * Turso DAG & Mistakes Ledger           * Time Awareness date guards    |
|   * SDLC phase-gate enforcement           * Compact markdown web fetch    |
|   * Blast-radius symbol tracing           * Knowledge graph indexing      |
+---------------------------------------------------------------------------+
```

- **[Kavach (`kavach-rs`)](https://github.com/Gaurav-Wankhede/kavach-rs):** Governs local codebase state, AST mutations, context firewalls, and persistent session memory.
- **[Webfind (`webfind`)](https://github.com/Gaurav-Wankhede/webfind):** Governs external web search, live documentation grounding, and URL extraction. It guarantees agents never rely on frozen training weights.

---

## The 12 Legacy Replacements

Kavach replaces the brittle, bloated POSIX and runtime tools commonly used by autonomous coding agents with fast, secure Rust implementations:

| Need / Intent | Legacy Tool (Banned) | Kavach Replacement | Architectural Guarantee |
| :--- | :--- | :--- | :--- |
| **Extract code slice** | `cat`, `head`, `tail` | `kavach grep "<symbol>" <path>` | **Zero whole-file reads; exact AST pattern match** |
| **Locate files** | `find`, `fd` | `kavach glob "<pattern>" <path>` | **Bounded depth search; avoids directory flood** |
| **Patch code / text** | `sed`, `awk`, whole-file write | `kavach replace '<old>' '<new>' <file>` | **Atomic Aho-Corasick replacement via tempfile** |
| **Read JSON state** | `cat`, `jq` | `kavach json -j '.<path>' <file>` | **Field projection only; never loads raw JSON** |
| **Scripting / Math** | `python3 -c`, `node -e` | `kavach script -e "<expr>"` | **In-process Rhai AST; 4.7ms vs 118ms (Node)** |
| **Symbol callers** | Manual grep loops | `kavach trace "<symbol>" <path>` | **Inbound & outbound call graph extraction** |
| **Code provenance** | `git blame` parsing | `kavach origin "<symbol>" <path>` | **Tracks author, commit, and intent for symbol** |
| **Task / Roadmap** | `cat ROADMAP.md` | `kavach db kanban --project <slug>` | **Relational DAG stored in embedded Turso DB** |
| **Quality gates** | Loose bash test scripts | `kavach verify [--phase <name>]` | **SDLC phase-gated enforcement engine** |
| **Working state** | Stale agent memory | `kavach working-state` | **Tracks open edits, modified files & dirty state** |
| **Mistakes ledger** | Lost agent history | `kavach db mistakes record` | **Anti-pattern store prevents repeating errors** |
| **Live Web Search** | Unchecked LLM memory | `webfind search --live "<query>"` | **Zero-cost live truth grounding with date awareness** |

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

Comparing standard developer actions performed via legacy POSIX tools, MCP servers, and Kavach surgical CLI extraction:

| Action / Query | Legacy / MCP Baseline | Baseline Tokens | Kavach Command | Kavach Tokens | Context Window Savings |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Tool Definition (Schema Overhead)** | MCP Server (JSON-Schema injected every turn) | 8,000–18,000 tokens / turn | Native Rust CLI Binary | **0 tokens / turn** | **100% Tax Elimination** |
| **Inspect single JSON key** | `cat AGENTS.json` (42 KB) | 9,299 tokens | `kavach json -j .witnesses AGENTS.json` | 66 tokens | **99.28% reduction** |
| **Locate Rust function signature** | `cat src/main.rs` (38 KB) | 8,450 tokens | `kavach grep "fn main" src/ -C 2` | 42 tokens | **99.50% reduction** |
| **Find symbol caller network** | `grep -rn "execute_task" .` | 1,820 tokens | `kavach trace "execute_task" .` | 58 tokens | **96.81% reduction** |
| **Evaluate math / array logic** | `python3 -c "print([...])"` | 185 tokens | `kavach script -e "[...].len()"` | 12 tokens | **93.51% reduction** |
| **Read project task queue** | `cat ROADMAP.md` (full file) | 3,400 tokens | `kavach db kanban --project my-app` | 110 tokens | **96.76% reduction** |

---

## Core Architecture & Capabilities

```
+---------------------------------------------------------------------------------------+
|                                KAVACH-RS ARCHITECTURE                                 |
|                                                                                       |
|  +--------------------+  +--------------------+  +---------------------------------+  |
|  |   kavach grep      |  |   kavach script    |  |            kavach db            |  |
|  | AST / regex filter |  | Rhai in-memory VM  |  |   Turso Embedded libSQL Engine  |  |
|  +---------+----------+  +---------+----------+  +----------------+----------------+  |
|            |                       |                              |                   |
|            +-----------------------+------------------------------+                   |
|                                    |                                                  |
|                                    v                                                  |
|                      [ DETERMINISTIC DISPATCH ENGINE ]                                |
|                                    |                                                  |
|            +-----------------------+------------------------------+                   |
|            |                       |                              |                   |
|            v                       v                              v                   |
|  +--------------------+  +--------------------+  +---------------------------------+  |
|  |  kavach replace    |  |   kavach trace     |  |          kavach verify          |  |
|  | Atomic Aho-Corasick|  | Call graph lineage |  |      SDLC Phase-Gate Engine     |  |
|  +--------------------+  +--------------------+  +---------------------------------+  |
+---------------------------------------------------------------------------------------+
```

### 1. Surgical Extraction Engine
- **Command:** `kavach grep "<pattern>" [path]` / `kavach glob "<pattern>" [path]`
- **Design:** Slices specific AST functions, traits, or types out of source files with bounded context (`-C 2`). Prevents agents from reading entire files into memory.

### 2. Deterministic Sandboxed Scripting
- **Command:** `kavach script -e "<rhai_code>"` / `kavach script <file.rhai>`
- **Design:** Rhai in-memory scripting engine compiled directly into the binary. Zero subprocess fork overhead, executing arbitrary array and mathematical logic in **4.7 ms** with zero dependencies.

### 3. Embedded Turso & Knowledge Graph Memory Substrate
- **Command:** `kavach db write`, `kavach db read`, `kavach db kanban`
- **Design:** Powered by Turso's embedded `libSQL` engine. Maintains structured session memory across agent disconnects:
  - **Memory Entries:** Ephemeral and permanent keys across categories (`decision`, `roadmap`, `research`, `pattern`).
  - **Dependency DAG:** Task cards linked with `--depends-on` edges, enforcing topological ordering.
  - **Mistakes Ledger:** Self-reported agent anti-patterns stored with `--gate`, `--banned`, and `--instead` flags.

### 4. Atomic AST Code Patching
- **Command:** `kavach replace '<old_content>' '<new_content>' <file_path>`
- **Design:** Multi-pattern atomic string replacement using the `aho_corasick` algorithm writing through `tempfile`. Eliminates accidental file truncation from broken sed or python scripts.

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

# Install directly to ~/.cargo/bin and clean build artifacts
cargo install --path crates/kavach-cli --locked
cargo clean
```

### Companion CLI: Webfind

Install [Webfind](https://github.com/Gaurav-Wankhede/webfind) for live, zero-cost external research grounding:

```bash
git clone https://github.com/Gaurav-Wankhede/webfind.git
cd webfind
cargo install --path . --locked
cargo clean
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
       +---> kavach json .subsystems config.json                 (Surgical Schema Projection)
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
[ PHASE 4: SDLC VERIFICATION & KNOWLEDGE PERSISTENCE ]
       |
       +---> kavach verify --phase unit-tests
       +---> kavach db write --category roadmap --depends-on ... (DAG Dependency Link)
       +---> kavach db mistakes record                           (Anti-Pattern Self-Report)
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
# Surgical JSON inspection (zero whole-file reads)
kavach json -k .universal_behaviour.imperatives prompts/AGENTS.json

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

### Phase 4: SDLC Verification, Knowledge Graph & Memory Checkpoint

The agent enforces the SDLC phase gate and commits decisions and dependency graph edges:

```bash
# 1. Enforce SDLC phase gate
kavach verify --phase pre-commit

# 2. Persist Architectural Decision Record (ADR)
kavach db write --project my-app --category decision --key paseto_auth \
  --title "Adopt PASETO v4 over JWT" --new \
  --content "Stateless security tokens with authenticated symmetric encryption."

# 3. Create dependent roadmap card linked in the DAG
kavach db write --project my-app --category roadmap --key impl_auth_middleware \
  --title "Implement Auth Middleware" --priority 1 \
  --depends-on paseto_auth --new \
  --exec-prompt "Wire PASETO validator into Axum router with zero-copy decoding"

# 4. Relate concepts in the cross-project knowledge graph
kavach db concept-add --name paseto_v4 --display "PASETO v4" --desc "Secure tokens"
kavach db concept-link --from paseto_v4 --edge alternative_to --to jwt

# 5. Self-report any errant behavior into the Mistakes Ledger
kavach db mistakes record \
  --gate assumed_without_verifying \
  --banned "Guessed crate flag without checking --help" \
  --instead "Always run --help or webfind live search first"

# 6. Execute unblocked roadmap cards in parallel
kavach db parallel-run --project my-app --max-parallel 3 --verify-cmd "cargo test"
```

---

## Canonical Multi-Agent Governance (`prompts/`)

Kavach includes battle-tested prompt specifications designed for production agent harnesses in the [`prompts/`](prompts/) directory:

- [`prompts/AGENTS.json`](prompts/AGENTS.json): **Single Source of Truth (SSOT).** Schema-validated machine configuration declaring toolbelt mappings, domain witnesses, universal imperatives, and allowlists.
- [`prompts/AGENTS.md`](prompts/AGENTS.md): **Human-Readable Action Imperative Index.** Markdown interface pointing directly into `AGENTS.json` via `kavach json` projections.
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
    "db_path": ".kavach/kavach.db",
    "categories": ["decision", "research", "roadmap", "pattern", "app_spec"]
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
