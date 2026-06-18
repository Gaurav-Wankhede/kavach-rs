# kavach-web — HTMX server replacing the Dioxus GUI

Status: PLAN (2026-06-18). Replaces the deleted `kavach-app` Dioxus desktop GUI
with a server-rendered HTMX web app on `http://127.0.0.1:777`.

## Why
The Dioxus desktop binary was being installed over `~/.local/bin/kavach`, so every
hook invocation spawned a webview window. GUI removed entirely; a browser-based
HTMX UI is lighter, has no AppKit/WebKit linkage, and never shadows the CLI.

## Hard constraint (drives the architecture)
SurrealDB's RocksDB backend is **single-process**. The `kavach-rpc` daemon holds
the DB open, so `kavach-web` must NOT open the DB directly. It talks to the running
daemon over the **Unix-socket JSON-RPC client** (`kavach_rpc::client::call`) — the
same path the old GUI used. This also yields live updates via `change.wait(since)`.

## Architecture
```
Browser ──HTTP/HTMX──► kavach-web (:777) ──unix-socket JSON-RPC──► kavach-rpc daemon ──► SurrealDB
        ◄──SSE (/events)──  loops on change.wait
```
- New crate `crates/kavach-web` (Axum 0.8, workspace dep to add).
- Rendering: **maud** (compile-time-checked HTML macros; no template dir, no build step).
- Interactivity: HTMX attributes; endpoints return HTML fragments.
- Live updates: one SSE endpoint `/events` loops on `change.wait`, emits an
  `hx-trigger` "refresh" event; pages carry `hx-trigger="sse:refresh"` to re-fetch
  their fragment. Replaces the GUI `REFRESH_TICK` signal.
- Static assets: `tower-http::ServeDir` (already a workspace dep) serves CSS +
  the Cytoscape/dagre JS (recover from git `HEAD:crates/kavach-app/assets/`).
- Launch: new `kavach web [--port 777]` subcommand starts the server. A thin
  "open" helper just launches the default browser at the URL — NO webview.

## RPC client wrapper
Reuse `kavach_rpc::client::call()` over the Unix socket (authenticated default,
no token). Mirror the old `rpc_client.rs` helpers:
- `rpc<P,R>(method, params) -> Result<R, RpcError>`
- `rpc_no_params<R>(method) -> Result<R, RpcError>`
Error variants: DaemonOffline, Rpc{code,message}, Io, Decode. On DaemonOffline,
render a friendly "daemon not running — `kavach daemon install` then bootstrap" page.

## Page → RPC method map (all methods already exist on the daemon)
| Page            | Method                              | Notes |
|-----------------|-------------------------------------|-------|
| Projects        | `db.list_projects`                  | landing + project selector |
| Roadmap         | `db.query(category=roadmap, all)`   | list + entry detail |
| Kanban          | `db.kanban(project,limit,status?)`  | 4-col board + DAG (Cytoscape) |
| Decisions       | `db.query(category=decision, all)`  | list |
| Knowledge graph | `graph.fetch(project, entity_type?)`| Cytoscape force layout |
| Concepts        | `concept.list` / `concept.search`   | list + add form (POST) |
| Mistakes        | `mistake.hit_count(name)`           | lookup |
| Runs            | (was in-memory only)                | needs persistent runs table — see Phase 6 |

## Writes (full-parity scope)
- Entry editor: HTML form GET (`/entries/:key/edit` → fragment) + POST
  (`/entries/:key` → `db.write`/update RPC), `hx-swap` the row back in.
- Add-concept form → `concept.add` (verify exact method name in methods/concept.rs).
- Optimistic UI not required; rely on SSE refresh after the write round-trips.

## Build phases (cards)
1. **Skeleton** — `crates/kavach-web` crate; Axum on :777; maud base layout +
   sidebar; unix-socket RPC wrapper; `kavach web` subcommand; daemon-offline page.
2. **Read pages A** — Projects, Roadmap, Kanban board, Decisions.
3. **SSE live updates** — `/events` on `change.wait`; wire `hx-trigger="sse:refresh"`.
4. **Read pages B** — Knowledge graph, Concepts, Mistakes (recover Cytoscape assets).
5. **Writes** — entry editor form POSTs (`db.write`), add-concept form.
6. **Runs (persistent)** — NEW backend: a `runs` table + RPC methods to spawn/track/
   cancel Claude Code subprocesses (the GUI tracked these only in memory). Largest
   piece; scope separately — confirm whether run-spawning belongs in the daemon.

## Verification per phase
- `cargo clippy --release -p kavach-web -- -D warnings` + build clean.
- Manual: `kavach web` → browser at :777 → each page renders against a live daemon.
- SSE: mutate a card via CLI (`kavach db status-update …`) → page updates without reload.

## Open question for Phase 6
Run-spawning (forking `claude` subprocesses, SIGTERM cancel, cost tracking) was a
GUI responsibility. For a web server this must move server-side and likely into the
daemon (persistent, survives browser reload). Decide ownership before building Phase 6.
