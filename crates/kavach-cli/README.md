# kavach-cli

The sovereign command-line interface, runtime harness governor, and developer workbench for the Kavach framework.

---

## Why kavach-cli Exists

Modern AI coding agents generate fast code, but without strict guardrails they introduce architectural drift, delete working code, fabricate unverified dependencies, and bypass test suites. 

`kavach-cli` (accessible as `kavach`) is the unified command-line tool that enforces SDLC phase gates, manages project Kanban tasks, executes deterministic polyglot searches, audits codebase symbols, and eliminates agent session amnesia.

---

## Key Features

- **Runtime SDLC Governance**: Enforces phase gates (`kavach verify`, `kavach gate`) so agents must research before writing code.
- **Memory & Kanban**: Built-in SQLite-backed task tracking and memory persistence (`kavach db`, `kavach kanban`).
- **Surgical Code Discovery**: AST-aware search and symbol inspection (`kavach grep`, `kavach glob`, `kavach trace`) designed for zero whole-file token bloat.
- **Atomic In-Place Patching**: High-throughput surgical replacements (`kavach replace`) powered by Aho-Corasick algorithms.
- **Session Continuity**: Eliminates agent context amnesia with working-state tracking (`kavach working-state`).

---

## Installation

Install `kavach-cli` directly via Cargo:

```bash
cargo install kavach-cli --locked
```

---

## Quick Start

```bash
# Initialize Kavach governance in a repository
kavach init

# Inspect project kanban tasks
kavach kanban list

# Check active phase gate compliance
kavach verify
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Toolbelt Sovereign**: Composes specialized, battle-tested Rust crates (`kavach-*`) into a unified binary.
- **Local First**: All database storage, session tracking, and rule evaluations happen 100% locally on your machine.
- **Zero LLM Hallucination in Core**: The governance engine and search tools are deterministic Rust code, never speculative model generations.

---

## License

Licensed under the Apache License, Version 2.0.
