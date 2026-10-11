# Autonomous Harness and Self-Healing

Kavach equips agents with closed-loop autonomous execution workflows.

---

## 1. Autonomous Loop (`kavach loop`)
Drives agents toward verified goal states without manual per-turn prompting:
- Monitors target condition or tests until fully satisfied.
- Enforces execution ceilings to prevent runaway costs.

```bash
kavach loop start --goal "make all 261 nextest tests pass"
```

---

## 2. Loophole Hunter (`kavach loophole`)
Runs scheduled sweeps across codebase vulnerability lenses via `launchd`:
- Scans security attack surfaces and missing invariants.
- Records findings into the mistake ledger before code reaches CI.

```bash
kavach loophole sweep
```

---

## 3. Self-Heal Ingestion Bridge (`kavach heal`)
Connects remote CI breakages directly into local developer sessions:
- `kavach heal ingest`: Polls open GitHub issues labeled `self-heal`, converting them into local roadmap task cards.
- `kavach heal merge-gate`: Fail-closed gate allowing merges only when CI is green, 3-witness passes, and protected paths are untouched.
