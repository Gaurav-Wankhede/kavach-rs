# Self-Healing Pipeline (Kavach replaces N8N)

Kavach is the orchestrator for a fully self-healing ecosystem. It does **only
non-AI ops** — capturing failure context, queuing/dispatching work, deciding
merge safety. **All AI fixing happens inside the subscription native agent** via
the autonomous loop; Kavach NEVER calls a metered LLM. This is the N8N→OpenAI
design re-homed onto the subscription-only constraint.

## The loop (H1–H5)

```
                        ┌──────────────────────── REACTIVE ────────────────────────┐
  CI `build` fails ─▶ self-heal.yml (workflow_run, conclusion==failure)             │
     (H2)              gathers `gh run view --log-failed` (non-AI)                   │
                       opens ONE `self-heal`-labelled GitHub Issue (idempotent/run)  │
                                              │                                       │
  host: `kavach heal ingest` (H5 bridge) ◀───┘                                       │
     polls open `self-heal` issues → `kavach heal capture` (H1, RPC single-writer)   │
     relabels issue `self-heal-queued` (exactly-once)                                │
                        └───────────────────────────────────────────────────────────┘

                        ┌──────────────────────── PROACTIVE ───────────────────────┐
  host cron: `kavach heal sweep` (H3)                                                │
     runs non-AI gates [cargo check / clippy -D warnings / cargo machete] --workspace│
     captures a card per failing gate (H1)                                           │
                        └───────────────────────────────────────────────────────────┘

  EITHER source writes a `heal.incident.*` roadmap card (status=todo)
        │
  autonomous loop dispatches the card ▶ SUBSCRIPTION native agent root-causes,
        fixes AT THE SOURCE, 3-witness verifies, opens a PR
        │
  `kavach heal merge-gate --pr N --witness-pass` (H4, fail-closed) decides:
        ALLOW only if KAVACH_HEAL_AUTOMERGE=1 AND CI green AND 3-witness AND
        no protected path — else DENY (human merges).
```

## Components

| Unit | Command / file | Role |
|------|----------------|------|
| H1 | `kavach heal capture` · `cmd/heal/capture.rs` | non-AI context gather → roadmap card (RPC single-writer, idempotent on incident id) |
| H2 | `.github/workflows/self-heal.yml` | reactive: CI failure → `self-heal` issue (injection-safe, idempotent/run) |
| H3 | `kavach heal sweep` · `cmd/heal/sweep.rs` | proactive: workspace gates → card per failure |
| H4 | `kavach heal merge-gate` · `cmd/heal/merge_gate/` | fail-closed auto-merge decision (default OFF) |
| H5 | `kavach heal ingest` · `cmd/heal/ingest/` | bridge: `self-heal` issues → local cards |

## Host scheduling (config, not code)

H2 fires itself (GitHub event). H3 (proactive sweep) and H5 (issue ingest) are
host-side and run on a schedule. Add to the host's launchd/cron (example,
every 30 min):

```
*/30 * * * *  cd <repo> && kavach heal sweep   --project kavach-rs
*/30 * * * *  cd <repo> && kavach heal ingest  --project kavach-rs
```

Auto-merge is OFF until you opt in with `KAVACH_HEAL_AUTOMERGE=1` in the loop's
environment; without it H4 always denies and a human performs every merge.

SOURCE: decision rows `heal.*` in kavach-db (project kavach-rs).
