# `kavach-config`

> **Deterministic, Layered Project & Global Configuration Engine for Autonomous AI Agent Governance.**

[![Crates.io](https://img.shields.io/crates/v/kavach-config.svg)](https://crates.io/crates/kavach-config)
[![Documentation](https://docs.rs/kavach-config/badge.svg)](https://docs.rs/kavach-config)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## What is `kavach-config`?

`kavach-config` is a type-safe, layered configuration management crate tailored for autonomous AI agent developer harnesses. It manages project boundaries, global rule directives, allowed toolbelts, and security stop-gate policies.

---

## Why Does It Exist? (The Problem It Solves)

AI coding agents regularly fail due to **Environment Drift**:
1. **Conflicting Configuration Sources:** Different tools look in different places (`.env`, `package.json`, flags, shell exports), leading agents to guess configuration paths.
2. **Untracked Policy Overrides:** When an agent works in a multi-repo workspace, it needs deterministic resolution of what tools are allowed per project without mutating global system security settings.
3. **Fragile Schemas:** Unvalidated JSON or YAML files cause runtime panics midway through a multi-step task execution.

`kavach-config` provides a Single Source of Truth (SSOT) hierarchy: **System Global -> User Preferences -> Local Project Overrides**, resolved deterministically at runtime with zero guessing.

---

## Behind the Curtain: Honest Architectural Disclosures

We believe in 100% transparency about our dependencies and design decisions:

- **Underlying Engines & Libraries:**
  - **[`serde`](https://crates.io/crates/serde) & [`serde_json`](https://crates.io/crates/serde_json):** Zero-copy deserialization and strict schema enforcement.
  - **[`toml`](https://crates.io/crates/toml):** Clean human-readable local configuration file parsing.
  - **[`dirs`](https://crates.io/crates/dirs):** Cross-platform standard resolution of system config directories (`~/.config`, `~/.kavach`, `%APPDATA%`).
- **What Kavach Builds on Top:**
  - **Deterministic Precedence Cascade:** Global system policies always supersede local agent overrides unless explicitly permitted by an authorized security gate.
  - Integration with `kavach-types` domain models to ensure compile-time safety across all Kavach runtime modules.
- **Tradeoffs & Non-Goals:**
  - Designed for low-churn, high-integrity governance configs. Not intended as a high-frequency real-time key-value store (use `kavach-db` for dynamic state).

---

## Key Capabilities

- **Strict Schema Validation:** Fails closed immediately on unrecognized fields or syntax errors.
- **Hierarchical Layering:** Seamlessly merges machine-level settings (`~/.gemini/settings.json`), user rules (`~/.kavach/config.json`), and repo-level rules (`.kavach/config.json`).
- **Zero Ambiguity:** Gives coding agents a reliable, single API call to determine active toolbelts and permissions.

---

## Quickstart

Add to your `Cargo.toml`:
```toml
[dependencies]
kavach-config = "1.3"
```

Resolve configuration in Rust:
```rust
use kavach_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_canonical()?;
    println!("Zero whole-file reads: {}", config.governance.zero_whole_file_reads);
    Ok(())
}
```

---

## Part of the Kavach Sovereign Engine

This crate is an official component of **[`kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)**, the sovereign context firewall and cognitive memory substrate for AI coding agents.

- Canonical Repository: [`https://github.com/Gaurav-Wankhede/kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)

---

## Author & License

- **Author & Architect:** [Gaurav Wankhede](https://github.com/Gaurav-Wankhede) (`gauravanilwankhede2002@gmail.com`)
- **License:** MIT License
