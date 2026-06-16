# Meta-Harness Loophole Loop

A self-healing loop that **finds loopholes in the system, records them in the
Kavach DB, and keeps hunting until the system converges.** A loophole is a defect
the happy path never exercises — so a clean build and a green test suite do *not*
prove its absence. Only an adversarial question does. This loop asks that question
mechanically, on a schedule, and at every stop.

> **Economics invariant.** Kavach only **DETECTS + RECORDS** via non-AI heuristics.
> It **never spawns an LLM** and never touches metered billing. Every *fix* happens
> inside the native subscription tool (the agent that picks up the recorded card).
> Subscription-only, not metered.

## The six attack lenses

Mirrors `~/.claude/CLAUDE.md` §`loophole_self_interrogation`. The shared kernel
`kavach_patterns::loophole_lens` runs every lens over a file's text:

| Lens | Failure mode the happy path never exercises |
|------|---------------------------------------------|
| `concurrency` | two actors at once → TOCTOU / lost-update / double-claim |
| `failure` | process dies mid-op → orphaned lock / half-write / leaked task |
| `malformed` | null / huge / wrong-type / hostile input → panic / injection |
| `authz` | caller without rights → missing check / confused-deputy / IDOR |
| `replay` | same request twice → non-idempotent mutation |
| `boundary` | empty / max / negative / off-by-one |

The kernel is **pure** (`scan_text(&str) -> Vec<LensFinding>`), conservative (a
hint, not a proof), and is the **one source of truth** consumed by both the CLI
sweep and the engine Stop-gate hook — they can never drift apart. Test code
(`tests.rs`, `*_test.rs`, `*_tests.rs`, anything under `tests/`, and everything
below a `#[cfg(test)]` marker) is excluded — it legitimately uses
`unwrap`/`expect`/index, so scanning it would flood the board with non-defects.

## YAML, not Markdown, for the loop pipeline

Each round emits a per-iteration **YAML** artifact to a fixed `/tmp` working dir,
`/tmp/kavach-loophole/<run-id>/iter-<round>.yaml`, so the unit of work is
precisely targeted, diffable, and machine-readable. (Research verdict: YAML for
deterministic/structural loop pipelines; Markdown for free-form intent specs —
`research.yaml-vs-markdown-meta-harness-loop`.) The DB (mistakes/patterns/roadmap)
is the durable learning store; the YAML is the per-round work order.

## Three triggers

The loop runs under all three, by design:

1. **On-demand CLI** — `kavach loophole sweep` (one round) and `kavach loophole
   loop` (loop-until-dry). For ad-hoc and operator-initiated hunts.
2. **Stop-gate hook** — every Stop that shipped risk-bearing work runs a *bounded*
   lens scan over the turn's git-changed files and queues a concrete
   `[LOOPHOLE_SITES]` advisory naming suspected `(lens, file:line)` sites. The
   teeth behind the prompt-only `[LOOPHOLE_CHECK]` self-interrogation.
3. **Proactive cron** — `kavach loophole cron` installs a code-owned launchd
   `LaunchAgent` (`ai.shared.kavach-loophole`) that runs `kavach loophole loop`
   daily. `RunAtLoad=false` so installing does not trigger an immediate sweep.

## Loop-until-dry convergence

`kavach loophole loop` re-runs sweep rounds, accumulating every `(lens, site)`
incident key in a set, until **`dry_rounds` consecutive rounds surface no NEW
key** (convergence) OR `max_rounds` is hit (the runaway brake). The native agent
fixes cards between rounds, shrinking the finding set until it goes dry.

- `cap = max(max_rounds, dry_rounds)` — a clean repo can always structurally
  converge (`max_rounds < dry_rounds` would otherwise make convergence impossible).
- Exit `0` on convergence, `1` on cap-without-convergence (never a false clean stop).
- Each round's findings are recorded idempotently per `(lens, file, line)` via the
  H1 RPC single-writer path — re-sweeping the same loophole updates one card.

```
kavach loophole loop --project P --run-id loop --dry-rounds 2 --max-rounds 10
```

## End-to-end proof

Plant → detect → fix → re-sweep clean → converge. Run on this repo:

1. **Plant** a synthetic loophole — an `unwrap()` on external input (malformed
   lens) in a temporary tracked `.rs` file.
2. **Detect** — the plant appears in the sweep's source set (`git ls-files`) and
   the kernel flags its `unwrap()` line.
3. **Record** — `kavach loophole loop` writes a heal card per finding (idempotent
   key `loophole-malformed-<path>-L<line>`).
4. **Fix** — remove the synthetic loophole (simulates the agent's root fix).
5. **Re-sweep clean** — the plant is gone from the source set; the synthetic site
   no longer fires. With no NEW site, the round is dry; `dry_rounds` dry rounds in
   a row ⇒ **CONVERGED** (exit 0).

The convergence accounting (`next_streak`, dry-round detection) is unit-tested in
isolation in `crates/kavach-cli/src/cmd/loophole.rs`; the kernel heuristics in
`crates/kavach-patterns/src/loophole_lens_test.rs`; the bounded Stop-gate scanner
in `crates/kavach-engine/src/gates/loophole_guard_tests.rs`.

## Code map

| Concern | Location |
|---|---|
| Shared lens kernel (pure) | `crates/kavach-patterns/src/loophole_lens.rs` |
| CLI per-file adapter | `crates/kavach-cli/src/cmd/loophole/detect.rs` |
| Sweep + loop-until-dry | `crates/kavach-cli/src/cmd/loophole.rs` |
| Per-iteration YAML model | `crates/kavach-cli/src/cmd/goal/loop_yaml/loophole.rs` |
| Proactive cron install | `crates/kavach-cli/src/cmd/loophole/cron.rs` |
| Stop-gate detector + nudge | `crates/kavach-engine/src/gates/loophole_guard.rs` |
| Stop-gate wiring | `crates/kavach-engine/src/gates/stop.rs` |

## Boundaries (no silent caps)

- Sweep records at most 50 cards/round; excess is **named**, not dropped.
- Stop-gate scans at most 24 changed files and lists at most 12 sites; both caps
  are surfaced in the advisory text.
- Every cap is a `const` with a one-line rationale at its definition.
