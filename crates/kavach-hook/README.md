# kavach-hook

Agent lifecycle hook interception, pre/post-tool call injection, and Claude Code protocol bridge.

---

## Why kavach-hook Exists

To govern AI coding agents without modifying their upstream source code, you need a non-intrusive hook interception mechanism. Intercepting execution at the tool-dispatch boundary allows policy engines to validate actions, enforce evidence chains, and inject runtime context before tools execute.

`kavach-hook` bridges host agent environments (including Claude Code and AGY) into the Kavach governance architecture. It intercepts pre-tool and post-tool lifecycle events, validates inputs, injects active context, and terminates unsafe executions cleanly.

---

## Key Features

- **Lifecycle Interception**: Clean hooks for session start, pre-tool call, post-tool call, and prompt submission.
- **Claude Code Integration**: Native compatibility with Claude Code 2.1 lifecycle protocols and stdin/stdout message schemas.
- **Context Injection**: Dynamically injects memory entries, active phase gates, and system invariants into agent prompts.
- **Input Sanitization**: Inspects and sanitizes tool arguments before they reach OS shells or file system writers.

---

## Installation

Add `kavach-hook` to your `Cargo.toml`:

```toml
[dependencies]
kavach-hook = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-hook
```

---

## Quick Start

```rust
use kavach_hook::lifecycle::HookHandler;

let handler = HookHandler::new();
// Evaluates tool invocation and injects governance context
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Hook Adapter**: Intercepts and transforms lifecycle messages between agent CLIs and Kavach engines. It does not replace the host agent runtime itself.
- **Sub-Millisecond Overhead**: Designed to add less than 2ms of overhead per tool invocation.
- **Strictly Local**: Zero external network requests during hook processing.

---

## License

Licensed under the Apache License, Version 2.0.
