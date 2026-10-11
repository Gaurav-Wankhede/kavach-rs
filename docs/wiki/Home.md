# Kavach Wiki

Welcome to the official developer documentation for **Kavach** — the sovereign in-process Rust development harness and cognitive memory bank for autonomous AI coding agents.

---

## Documentation Index

### 1. Getting Started
- [[Quickstart & Installation|Quickstart-and-Installation]]: 1-line installation, toolchain provisioning, and updates.
- [[Agent Harness Integration|Harness-Integration-Guide]]: Step-by-step setup for Claude Code, Cursor, Codex, and OpenCode.

### 2. Core Architecture
- [[Cognitive Memory & Brain-OS|Cognitive-Memory-and-Brain-OS]]: Embedded SQLite storage, BM25 + Graph Reciprocal Rank Fusion ($k=60$).
- [[Sovereign Toolbelt Reference|Sovereign-Toolbelt-Reference]]: In-process CLI proxies (`grep`, `glob`, `trace`, `json`, `replace`, `cargo`, `script`).
- [[Autonomous Harness & Self-Healing|Autonomous-Harness-and-Self-Healing]]: Bounded execution loops (`kavach loop`), loophole hunts (`kavach loophole`), and CI failure ingestion (`kavach heal`).

---

## Architectural Philosophy

AI coding agents are powerful but prone to four fundamental failure modes:
1. **Stateless Amnesia**: Agents lose context between sessions and re-litigate solved architectural decisions.
2. **Context Window Rot**: Dumps from legacy tools (`cat`, `grep`) flood models with irrelevant tokens.
3. **Hallucinated Pipelines**: Complex shell pipes (`|`) fail unpredictably.
4. **Premature Surrender**: Reporting "BLOCKED" on compiler warnings rather than autonomously rectifying them.

Kavach wraps every agent lifecycle event inside a **deterministic, sub-millisecond Rust verification layer**.
