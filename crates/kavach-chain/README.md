# kavach-chain

Intent classification, feature extraction, and evidence-chain enforcement for agent execution pipelines.

---

## Why kavach-chain Exists

Agents frequently jump straight to editing code without researching root causes or proving intent. Once code modification begins prematurely, debugging spirals out of control and context windows fill with noise.

`kavach-chain` enforces the Evidence-Chain protocol. It parses user intent, extracts semantic features, checks whether prerequisite research has been conducted, and locks file write tools until valid proof citations are recorded in the active context.

---

## Key Features

- **Evidence-Chain Enforcement**: Enforces the Research-First protocol before any file mutation is authorized.
- **Intent Feature Extraction**: Extracts intent classifications and action vectors from user prompts and tool calls.
- **Context Output Formatting**: Formats verified context and gate results for seamless agent injection.
- **Deterministic Gates**: Blocks write tools deterministically until evidence criteria are satisfied.

---

## Installation

Add `kavach-chain` to your `Cargo.toml`:

```toml
[dependencies]
kavach-chain = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-chain
```

---

## Quick Start

```rust
use kavach_chain::chain_state::ChainState;

let state = ChainState::new();
// Evaluates intent features and validates evidence chain
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Gate Protocol**: Enforces pre-condition verification rules; it is not an LLM itself, but a deterministic supervisor of LLM actions.
- **Zero-Latency Design**: Built for sub-millisecond evaluation in pure Rust.
- **Safety**: Fully safe Rust with robust error handling.

---

## License

Licensed under the Apache License, Version 2.0.
