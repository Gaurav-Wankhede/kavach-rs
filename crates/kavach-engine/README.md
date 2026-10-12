# kavach-engine

Core governance engine, phase gate runner, and deterministic verification governor for the Kavach framework.

---

## Why kavach-engine Exists

Complex software development demands strict phase discipline: planning before architecture, research before writing code, and verification before marking tasks complete. Autonomous agents routinely violate these disciplines when given unconstrained access.

`kavach-engine` is the central orchestrator that binds together rules, database state, hook lifecycles, and verification gates. It evaluates gate requirements, drives the Plan-Execute-Verify (PEV) loop, and halts execution whenever an agent attempts to skip required validation phases.

---

## Key Features

- **Gate Runner**: Enforces strict SDLC phase gates (Research Gate, Test Gate, Verification Gate).
- **Core Orchestrator**: Unifies `kavach-db`, `kavach-rule-engine`, and `kavach-session` into a single cohesive runtime.
- **Deterministic Verification**: Replaces LLM self-evaluation with rigorous compiler checks, linters, and test runners.
- **Failure Isolation**: Isolates gate evaluation errors so that system crashes cannot bypass security policies.

---

## Installation

Add `kavach-engine` to your `Cargo.toml`:

```toml
[dependencies]
kavach-engine = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-engine
```

---

## Quick Start

```rust
use kavach_engine::gate_runner::GateRunner;

let runner = GateRunner::new();
// Evaluates active gates and runs verification pass
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Orchestration Layer**: Executes deterministic gate logic by coordinating sub-crates; it does not replace standard Rust build tools (Cargo, Clippy).
- **Zero Hidden Telemetry**: All execution tracking is written strictly to local session files.
- **High Performance**: Optimized for instant in-process execution.

---

## License

Licensed under the Apache License, Version 2.0.
