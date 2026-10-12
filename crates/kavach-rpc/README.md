# kavach-rpc

JSON-RPC 2.0 in-process dispatch and client transport layer for the Kavach persistent database server.

---

## Why kavach-rpc Exists

Multiple Kavach CLI commands, background daemons, and browser UI dashboards must access the underlying Kavach memory store concurrently without corrupting disk files or causing SQLite lock contention.

`kavach-rpc` provides a unified JSON-RPC 2.0 transport client. It standardizes request encoding, response schemas, and error propagation across local socket connections and persistent database servers.

---

## Key Features

- **JSON-RPC 2.0 Compliance**: Strict conformance to the JSON-RPC 2.0 specification for requests, responses, batch calls, and notifications.
- **In-Process & Socket Dispatch**: Supports high-speed local IPC and in-process message routing with minimal serialization cost.
- **Typed Method Contracts**: Strongly typed request parameters and result payloads for all Kavach database operations.
- **Robust Error Mapping**: Maps database and protocol errors to canonical RPC error codes with clear diagnostic messages.

---

## Installation

Add `kavach-rpc` to your `Cargo.toml`:

```toml
[dependencies]
kavach-rpc = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-rpc
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Transport & Dispatch Layer**: Handles RPC serialization and dispatch; does not store data itself.
- **Local First**: Built primarily for local IPC between Kavach tools on developer machines.
- **Zero Unsafe**: Pure safe Rust with structured error handling.

---

## License

Licensed under the Apache License, Version 2.0.
