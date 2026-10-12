# `kavach-toon`

> **Token-Optimized Object Notation (TOON): Zero-Bloat Tabular JSON Projection for LLM Context Windows.**

[![Crates.io](https://img.shields.io/crates/v/kavach-toon.svg)](https://crates.io/crates/kavach-toon)
[![Documentation](https://docs.rs/kavach-toon/badge.svg)](https://docs.rs/kavach-toon)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## What is `kavach-toon`?

`kavach-toon` is a high-performance, token-efficient serialization format and formatting engine that converts verbose JSON object streams into compact, schema-aligned tabular representations (TOON) designed specifically for LLM attention heads.

---

## Why Does It Exist? (The Problem It Solves)

Standard JSON is notoriously inefficient for LLM prompt contexts:
1. **Repeated Key Token Waste:** In an array of 50 objects, repeating keys like `{"id": "...", "status": "...", "category": "..."}` 50 times inflates context consumption by up to 70%.
2. **Context Window Saturation:** Agents hit prompt truncation prematurely simply by inspecting structured database or schema states.
3. **Parse Friction:** LLMs read tabular row-oriented schemas with higher semantic accuracy and lower latency than deeply nested JSON brackets.

`kavach-toon` replaces repetitive JSON arrays with a single header schema line followed by tab-separated value rows, reducing token overhead by up to **80%** while preserving 100% of the structured data.

---

## Behind the Curtain: Honest Architectural Disclosures

We believe in complete architectural transparency:

- **Underlying Engines & Libraries:**
  - **[`serde_json`](https://crates.io/crates/serde_json):** Industry-standard, zero-copy JSON parsing.
  - **[`thiserror`](https://crates.io/crates/thiserror):** Type-safe error definitions for malformed tabular conversions.
- **What Kavach Builds on Top:**
  - The **TOON Tabular Serializer:** A deterministic row-packing algorithm that scans JSON value trees, automatically discovers consistent tabular columns, and serializes them into compact format.
- **Tradeoffs & Non-Goals:**
  - Designed for flat or semi-flat object arrays and key projections. Deeply nested polymorphic recursive structures should use standard compact JSON (`-j`).

---

## Quickstart

Add to your `Cargo.toml`:
```toml
[dependencies]
kavach-toon = "1.3"
```

Format JSON to zero-bloat TOON:
```rust
use kavach_toon::to_toon;
use serde_json::json;

fn main() {
    let data = json!([
        {"slug": "core", "status": "ok"},
        {"slug": "db", "status": "active"}
    ]);
    let toon = to_toon(&data).unwrap();
    println!("{}", toon);
}
```

---

## Part of the Kavach Sovereign Engine

This crate is the token-compression engine of **[`kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)**, the sovereign context firewall and cognitive memory substrate for AI coding agents.

- Canonical Repository: [`https://github.com/Gaurav-Wankhede/kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)

---

## Author & License

- **Author & Architect:** [Gaurav Wankhede](https://github.com/Gaurav-Wankhede) (`gauravanilwankhede2002@gmail.com`)
- **License:** MIT License
