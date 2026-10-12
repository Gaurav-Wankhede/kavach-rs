# kavach-web

Lightweight, server-rendered HTMX web interface and live dashboard for the Kavach agent memory store.

---

## Why kavach-web Exists

While terminal commands are essential for automated agent loops, human developers need visual dashboards to inspect Kanban boards, audit mistake ledgers, browse memory entries, and visualize rule trees without running heavy frontend build chains.

`kavach-web` is a self-contained, server-rendered web dashboard built on Axum and HTMX. It compiles into a compact binary, starts instantly on localhost, and renders live operational state directly from the Kavach database with zero JavaScript client build steps.

---

## Key Features

- **Zero-Build Frontend**: Built with HTMX and clean HTML templates; requires zero Node.js, npm, or webpack bundles.
- **Live Memory Inspector**: Interactive browser for Kanban cards, sessions, memory entries, and gate statuses.
- **Fast Axum Backend**: Asynchronous HTTP server powered by Tokio and Axum for instant local page loads.
- **Safe & Local**: Binds strictly to localhost (`127.0.0.1`) with unprivileged access for local developer privacy.

---

## Installation

Add `kavach-web` to your `Cargo.toml`:

```toml
[dependencies]
kavach-web = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-web
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Local Dashboard Only**: Designed for local development inspection on developer workstations; not a multi-tenant cloud SaaS frontend.
- **Thin Client**: Connects as a client to the local Kavach store; does not embed or manage separate storage backends.
- **Privacy**: No telemetry, no third-party CDN scripts, and zero tracking cookies.

---

## License

Licensed under the Apache License, Version 2.0.
