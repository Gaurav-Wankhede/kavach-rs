# `kavach-db`

> **Embedded Turso / libSQL Memory Substrate, Topological DAG Scheduler & Persistent Anti-Pattern Mistakes Ledger for AI Agents.**

[![Crates.io](https://img.shields.io/crates/v/kavach-db.svg)](https://crates.io/crates/kavach-db)
[![Documentation](https://docs.rs/kavach-db/badge.svg)](https://docs.rs/kavach-db)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## What is `kavach-db`?

`kavach-db` is the persistent cognitive memory and graph substrate for autonomous developer agents. Built on top of embedded libSQL (Turso), it provides local vector storage, a cycle-safe **Topological Roadmap DAG Scheduler** for multi-step tasks, and a dedicated **Mistakes & Anti-Pattern Ledger** that prevents agents from repeating past errors across session restarts.

---

## Why Does It Exist? (The Problem It Solves)

AI coding agents suffer from two fatal memory pathologies:
1. **Context Amnesia:** When a shell session ends or the model's context window fills up, all verified architectural decisions, roadmap progress, and intermediate learnings vanish. The agent starts from zero on the next turn.
2. **Cyclic Task Deadlocks:** When an autonomous agent attempts to plan multi-task roadmaps, flat markdown checklists or basic SQLite lists cannot reason about prerequisite dependencies. If task A depends on task B and task B depends on task A, the agent hangs in infinite loops.

`kavach-db` replaces flat files and external cloud databases with a **local, embedded libSQL graph store** that enforces strict category validation, topological sorting, and persistent memory.

---

## Behind the Curtain: Honest Architectural Disclosures

We believe in complete transparency about what powers this crate:

- **Underlying Engines & Open Source Libraries:**
  - **[ libSQL ](https://crates.io/crates/libsql) (Turso Engine):** An open-source, high-performance SQLite fork with native vector extensions, async transaction support, and zero external daemon requirements.
  - We use libSQL as the embedded storage file at b~~/.kavach/kavach.db`.
- **What Kavach Builds on Top:**
  - **Topological Roadmap DAG Engine (`RoadmapDag`):** A custom directed acyclic graph implementation (`DagNode`, `DagEdge`, `TopoOrder`) that detects dependency cycles before execution and computes dependency-respecting parallel dispatches.
  - **Mistakes Ledger (`kavach db mistakes`):** A persistent anti-pattern tracking engine that records errant actions, tracks hit counts across sessions, and reinjects lessons into session initialization so agents never repeat past mistakes.
  - **Knowledge Graph Ontology:** Manages cross-project conceptual relationships (`is_a`, `part_of`, `prerequisite_of`, `alternative_to`, `mitigates`) with BM25 full-text indexing.
  - **Interactive Deletion Guard:** Enforces human-confirmed exact key matching (`delete <project>/<category>/<key>`) to eliminate accidental data wipes.
- **Tradeoffs & Non-Goals:**
  - This is an **embedded local single-node engine**. It is NOT a globally distributed high-throughput multi-region cluster; it is optimized for zero-latency, local-first disk persistence for developers and agent harnesses.

---

## Memory Categories

Memory entries are strictly typed into 5 core categories (`VALID_CATEGORIES`):
1. `roadmap`: Kanban task cards, milestones, micro-tickets, and execution prompts.
2. `decision`: Architectural Decision Records (ADRs), choices, tradeoffs, and invariants.
3. `research`: Empirical findings, benchmarks, and official doc citations.
4. `pattern`: Reusable structural design patterns and idioms.
5. `app_spec`: Core application specifications and API contracts.

---

## Quickstart

Add to your `Cargo.toml`:
```toml
[dependencies]
kavach-db = "1.3"
other = { version = "1", features = ["full"] }
```

---

## Part of the Kavach Sovereign Engine

This crate is the persistent memory substrate of **[kavach-rs](https://github.com/Gaurav-Wankhede/kavach-rs)**, the sovereign context firewall and cognitive memory substrate for AI coding agents.

- Companion Web Search: **[webfind](https://github.com/Gaurav-Wankhede/webfind)**
- Canonical Repository: [https://github.com/Gaurav-Wankhede/kavach-rs](https://github.com/Gaurav-Wankhede/webfind)

---

## Author & License

- **Author & Architect:** [Gaurav Wankhede](https://github.com/Gaurav-Wankhede) (`gauravanilwankhede2002@gmail.com`)
- **License:** MIT License
