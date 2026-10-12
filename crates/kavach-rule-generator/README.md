# kavach-rule-generator

Automated pattern detection and rule generation for AI agent workflows, codebases, and tool execution patterns.

---

## Why kavach-rule-generator Exists

Manually writing dozens of governance rules for every project is slow and error-prone. Engineers often miss recurring failure modes, fragile shell invocations, or unvetted library patterns until a subagent repeats the mistake.

`kavach-rule-generator` analyzes repository structure, tool traces, and commit histories to automatically detect anti-patterns and emit structured Kavach governance rules. It turns recurring developer corrections into permanently enforced, machine-checked policy ASTs.

---

## Key Features

- **Pattern Detection**: Identifies banned tool usage, unvalidated web scraping, and redundant dependencies (`detect_patterns`).
- **Template-Driven Emission**: Generates canonical markdown and TOML rule definitions following the Kavach AST specification.
- **Continuous Learning**: Analyzes past session mistake ledgers to generate self-healing governance rules.
- **Zero Hallucination**: Emits strictly validated, compiler-checked rule blocks.

---

## Installation

Add `kavach-rule-generator` to your `Cargo.toml`:

```toml
[dependencies]
kavach-rule-generator = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-rule-generator
```

---

## Quick Start

```rust
use kavach_rule_generator::{detect_patterns, PatternType};
use std::path::Path;

let patterns = detect_patterns(Path::new("./src")).expect("Detection complete");
for pattern in patterns {
    println!("Detected pattern: {:?}", pattern);
}
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Heuristic & AST Detection**: Relies on deterministic syntax matching and codebase scanning. It does not call external LLMs or generate nondeterministic text.
- **Emission Targets**: Outputs standardized templates for consumption by `kavach-rule-parser` and `kavach-rule-storage`.
- **Zero Telemetry**: All analysis runs 100% locally on your machine.

---

## License

Licensed under the Apache License, Version 2.0.
