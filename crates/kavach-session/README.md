# kavach-session

Stateful session lifecycle management, working-memory persistence, and mistake ledger graphs for autonomous AI agents.

---

## Why kavach-session Exists

Autonomous agent sessions suffer from session amnesia: agents forget prior corrections, repeat failed terminal invocations, and lose context across prompt turns. Relying solely on raw LLM conversation context windows wastes tokens, increases costs, and guarantees eventual regression.

`kavach-session` provides deterministic state persistence for active agent sessions. It records execution phases, manages working-state files, maintains a mistake ledger graph to prevent repeated blunders, and tracks reward outcomes across tool invocations.

---

## Key Features

- **Mistake Ledger**: Records and graphs execution blunders, preventing agents from re-attempting known bad solutions.
- **Working-State Persistence**: Serializes live working memory, active phase gates, and scratchpad data into atomic local storage.
- **Turn Shadow & Relays**: Flushes and syncs turn snapshots for offline post-mortem analysis and learning loops.
- **Path Isolation**: Manages canonical session directories, temporary state files, and lockfiles with zero multi-process race conditions.

---

## Installation

Add `kavach-session` to your `Cargo.toml`:

```toml
[dependencies]
kavach-session = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-session
```

---

## Quick Start

```rust
use kavach_session::state::SessionState;

let session = SessionState::load_or_create(".kavach/session.json").expect("Session loaded");
println!("Active session ID: {}", session.id());
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Local Persistence**: Stores session state directly on disk using structured JSON/SQLite formats. No cloud sync or third-party telemetry.
- **Deterministic Recording**: Records facts and outcomes strictly as observed; does not synthesize or hallucinate speculative state.
- **Lightweight**: Zero background threads or daemons when idle.

---

## License

Licensed under the Apache License, Version 2.0.
