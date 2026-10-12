# `kavach-types`

> **Canonical Domain Models, AST Data Schemas & Strongly Typed Protocols for the Kavach Sovereign Engine.**

[![Crates.io](https://img.shields.io/crates/v/kavach-types.svg)](https://crates.io/crates/kavach-types)
[![Documentation](https://docs.rs/kavach-types/badge.svg)](https://docs.rs/kavach-types)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## What is `kavach-types`?

`kavach-types` is the core foundation crate of the Kavach sovereign engine. It defines the shared, strongly typed data models used across every layer of Kavach—including AST positions, security stop-gates, topological DAG nodes, Turso database schemas, event streams, and error boundaries.

---

## Why Does It Exist? (The Problem It Solves)

In complex developer tooling and autonomous multi-agent systems, subsystems frequently drift:
1. **Schema Divergence:** When background daemons, CLI clients, and database stores maintain independent data models, JSON serialization inconsistencies lead to subtle runtime crashes.
2. **Untracked Error Context:** Traditional generic error types (`anyhow::Error` or strings) strip semantic context, making it impossible for autonomous agents to diagnose *why* a phase gate failed.
3. **Memory Deserialization Overhead:** Inefficient types waste CPU cycles and inflate agent context token windows.

`kavach-types` solves this by establishing a **Single Source of Truth (SSOT)** for all data structures, guaranteeing zero schema drift across the entire workspace.

---

## Behind the Curtain: Honest Architectural Disclosures

We believe in complete transparency about what powers this crate:

- **Underlying Engines & Open Source Libraries:**
  - **[`serde`](https://crates.io/crates/serde) & [`serde_json`](https://crates.io/crates/serde_json):** Zero-copy, high-speed serialization and deserialization.
  - **[`strum`](https://crates.io/crates/strum):** High-performance macros for working with typed enums (string conversion, iteration, and property lookups).
- **What Kavach Builds on Top:**
  - The canonical schemas for **Cognitive Memory Entries** (`MemoryEntry`, `Category`, `EntryStatus`).
  - Strict security gate verdicts (`GateVerdict`, `StopReason`, `Violation`).
  - AST code slice coordinates (`Span`, `Position`, `SliceMetadata`).
- **Tradeoffs & Non-Goals:**
  - This is a **pure data and traits crate**. It contains zero I/O operations, zero database drivers, and zero network calls, ensuring near-instant compile times.

---

## Key Capabilities

- **Strict Enum Category Guarantees:** Ensures memory keys map to valid domain categories (`roadmap`, `decision`, `research`, `pattern`, `app_spec`).
- **Comprehensive Error Boundaries:** Strongly typed error definitions that give AI agents structured, machine-parseable feedback when a mutation is rejected.
- **High-Performance Serde Derives:** Ready for instant JSON-RPC and database persistence.

---

## Quickstart

Add to your `Cargo.toml`:
```toml
[dependencies]
kavach-types = "1.3"
```

Use strongly typed domain models in Rust:
```rust
use kavach_types::memory::{Category, EntryStatus};

fn main() {
    let cat = Category::Decision;
    assert_eq!(cat.as_str(), "decision");

    let status = EntryStatus::Verified;
    println!("Status is: {:?}", status);
}
```

---

## Part of the Kavach Sovereign Engine

This crate is the foundational type system of **[`kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)**, the sovereign context firewall and cognitive memory substrate for AI coding agents.

- Companion Web Search: **[`webfind`](https://github.com/Gaurav-Wankhede/webfind)**
- Canonical Repository: [`https://github.com/Gaurav-Wankhede/kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)

---

## Author & License

- **Author & Architect:** [Gaurav Wankhede](https://github.com/Gaurav-Wankhede) (`gauravanilwankhede2002@gmail.com`)
- **License:** MIT License
