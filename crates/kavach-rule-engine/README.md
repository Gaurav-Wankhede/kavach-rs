# kavach-rule-engine

High-throughput, deterministic compliance and enforcement engine for agent governance rules and phase gates.

---

## Why kavach-rule-engine Exists

Defining rules is meaningless without a sub-millisecond evaluation engine that enforces them before every agent action. Checking rules must never slow down interactive agent conversations or tool pipelines.

`kavach-rule-engine` executes policy matching, file scope validation, and compliance checks against active agent tool calls. It evaluates trigger conditions, enforces fail-closed gates, and emits actionable policy violation diagnostics before a destructive command can touch your system.

---

## Key Features

- **Deterministic Compliance**: High-speed evaluation of agent tool dispatches against active governance policies.
- **File & Path Matchers**: High-performance glob and path boundary checks to prevent unauthorized file mutations.
- **Context Awareness**: Evaluates rule conditions against live agent execution state, session depth, and phase boundaries.
- **Fail-Closed Default**: If an assertion is ambiguous or evaluation fails, execution halts safely by default.

---

## Installation

Add `kavach-rule-engine` to your `Cargo.toml`:

```toml
[dependencies]
kavach-rule-engine = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-rule-engine
```

---

## Quick Start

```rust
use kavach_rule_engine::compliance::ComplianceEngine;

let engine = ComplianceEngine::new();
// Evaluates tool invocation against loaded rules
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Enforcement Boundary**: Evaluates rule ASTs and emits compliance verdicts. It does not spawn daemon processes or intercept OS syscalls directly.
- **Pure Evaluation**: Built for sub-millisecond in-memory checking with zero network dependencies.
- **Safety**: Fully safe Rust memory safety with zero unsafe blocks.

---

## License

Licensed under the Apache License, Version 2.0.
