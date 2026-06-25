# AUTONOMOUS HARNESS SDLC — `exec_prompt` authoring template

Opus authors this into a roadmap card's `exec_prompt`. The pipeline serves it
verbatim (`kavach db next-prompt --project X`) to a cheaper executor — Haiku via
Claude Code or Composer 2.5 via Cursor. The executor has NO conversation context;
the prompt must be fully self-contained.

## Law: every `exec_prompt` is a closed work order

The executor cannot ask questions. If a fact is missing, the executor guesses —
and a guess is a defect. So the prompt carries the whole job: what, where,
constraints, proof, done.

## The seven-block shape (fill every block)

```
ROLE: You are a <stack> engineer executing ONE bounded task. No scope beyond it.

TASK: <single imperative sentence — the exact change to make>

FILES: <each file:symbol the change touches, with absolute paths>
  - crates/foo/src/bar.rs:fn baz — <what changes here>

CONSTRAINTS (non-negotiable, from the workspace laws):
  - Nano-files: one functionality per file; no 2+ consecutive comment lines.
  - Reuse before writing: rg/fd for an existing symbol first.
  - Strict lints stay on; never #[allow]. A ceiling is #[expect(reason=...)].
  - RCA before any fix-edit: symptom · 5 whys · root_cause · fix_strategy.
  - <stack-specific: e.g. no unwrap/expect/panic in production paths>

VERIFY (the executor MUST run these and paste output):
  - <build cmd, e.g. cargo clippy -p foo --all-targets → exit 0>
  - <test cmd, e.g. cargo nextest run -p foo → N passed>
  - git diff --stat → the expected files changed

DONE WHEN: <the single observable condition — all three witnesses green>

ON FAILURE: stop, paste the failing output, do NOT mark done. Do not invent a
workaround that suppresses the check.
```

## Authoring rules for Opus

- One card = one task. If the work needs two verify gates, it is two cards.
- Name real `file:symbol` targets — never "the relevant file". Opus has the
  repo; the executor does not.
- Put the verify command the executor can copy-paste, with the pass threshold.
- The DONE WHEN line is the oracle. It must be checkable by the executor alone.
- Keep it terse. The executor reads it as an instruction, not an essay.

## Pipeline contract

- Opus writes: `kavach db write --new --project X --category roadmap --key k \
  --title "..." --content "<human rationale>" --exec-prompt "<the seven blocks>"`.
- Executor consumes: `kavach db next-prompt --project X | claude -p --model claude-haiku-4-5`
  (or paste into Cursor Composer 2.5).
- `next-prompt` serves the top-priority `todo` card; empty exec_prompt → stderr
  error + exit 1 (you never feed an empty prompt to a model).

SOURCE: decision.roadmap-exec-prompt-pipeline; Claude Code headless —
https://code.claude.com/docs/en/headless
