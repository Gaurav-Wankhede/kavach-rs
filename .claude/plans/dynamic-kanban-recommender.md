# Plan: Dynamic relevance-ranked harness — kanban recommender + continuous DB + tri-agent triggers

## Goal (user's words)
Inject only RELEVANT kanban tasks (recommendation-system style) not push-all; DB updated
continuously (mistakes, loopholes); trigger tri-agents for quality always; precisely map hooks.
Research-pinned to SurrealDB **3.1.4** (Cargo.lock FACT).

## Research facts (cited)
- SurrealDB 3.1.4 native ranking: `search::score(n)` BM25 over `@@`/`@n@` FTS predicates
  (surrealdb.com/docs/surrealql/functions/database/search); KNN `<|N,metric|>` + vector::*
  (vector docs) — but vectors were REMOVED here (decision/onnx-removal-dag-rlaif-only); keep keyword+graph.
- Recommender ALREADY EXISTS: `KavachBrain::search` = BM25-FTS ⊕ graph-proximity → RRF
  (`crates/kavach-surreal/src/brain.rs`, `rrf.rs`), exposed as `brain.think` RPC.
- Current kanban injector emits census + ONE next card by PRIORITY (not relevance):
  `crates/kavach-engine/src/gates/intent/context.rs:181-228`.
- Precedent just shipped: 3 Mermaid injectors thread prompt→brain.think→focus w/ empty-focus guard
  (decision.context-rot.precise-graph-injection).

## Decisions (user-approved this turn)
- Tri-agent triggers = **advisory inject** (never block), mirrors `[INVOKE_AGENT]`.
- Ranking = **new `db.kanban_ranked` RPC verb** (typed, testable).

## Steps (test-first, 4-witness each)

### Phase 1 — db.kanban_ranked RPC (worker)
1. `crates/kavach-rpc/src/methods/db/flow.rs` (or new `kanban_rank.rs`): `KanbanRankedParams{project_slug, prompt, limit?}`
   → `KanbanRankedResult{cards: Vec<{key,title,status}>}`. Logic: empty prompt ⇒ priority-order
   (existing `roadmap_dag`/list path, session-start whole-board); non-empty ⇒ brain.think(prompt)
   ranked, filter id `roadmap.*` AND status∈{todo,in_progress}, truncate top-K (K=6 default,
   `gate.kanban_ranked_limit` overridable). Fail-soft: daemon/empty ⇒ empty list, never error.
2. Register `db.kanban_ranked` in `crates/kavach-rpc/src/rpc.rs`. Unit test the rank+filter+cap.

### Phase 2 — injector swap (worker)
3. `context.rs:185-228`: `append_live_kanban` takes `prompt`; call `db.kanban_ranked`; render
   relevance-ranked top-K runnable cards instead of single next. Empty-prompt path = today's
   census+next. Thread `prompt` from the existing `append_mermaid_views` call site (same prompt).
   Update the 3 render tests.

### Phase 3 — continuous DB: loophole ledger (worker)
4. `crates/kavach-session/src/loophole_ledger.rs` (NEW, mirror mistake_ledger.rs): `record_loophole(lens,file_line,markers)`
   → `kavach db write --category pattern --key pattern.loophole.<lens>.<sig8>`. Test-first.
5. Wire persist in `post_write_checks.rs` after `loophole_guard` fires (currently advisory-only).
6. SessionStart reinject recent loopholes (mirror mistake reinject), age-out > N turns.

### Phase 4 — tri-agent advisory triggers (worker)
7. PreWrite post-gates: emit `[INVOKE_WORKER]` when all guards pass (bounded-edit confidence).
8. Stop clean-exit: emit `[INVOKE_VERIFIER]` when a completion claim lacks any of the 3 witnesses
   (git diff / build / DB write). Advisory only — never blocks (per user). Thinker stays on the
   existing intent-time `[INVOKE_AGENT]` design dispatch.

### Phase 5 — verify + deploy (verifier)
9. 4-witness: clippy clean, nextest green (new tests for rank/filter/cap, loophole ledger roundtrip,
   trigger-fire conditions), rg live-caller proof, `kavach deploy` (makes it LIVE — hooks run installed bin).
10. Persist decision row `decision.harness.dynamic-relevance-injection` + update hook-map in
    `crates/kavach-engine/CLAUDE.md` Wiring Map. Re-run `audit_wiring.sh` to surface NEW orphans.

## Hook map (deliverable)
| Hook | Fires | RPC | Dynamic behavior |
|---|---|---|---|
| SessionStart | mermaid views + kanban (whole-board) + loophole reinject | brain.think("") + kanban_ranked("") + loophole_ledger | empty prompt ⇒ whole-spine |
| UserPromptSubmit(intent) | mermaid views + kanban_ranked(prompt) + INVOKE_AGENT(thinker) | brain.think(prompt) + db.kanban_ranked | top-K relevance; no-hit ⇒ omit |
| PreWrite | guard chain + INVOKE_WORKER (post-pass) | none | advisory on gate-pass |
| PostWrite | concept scan + loophole persist | concept.add + loophole_ledger | persist on fire |
| Stop | 3-witness check + INVOKE_VERIFIER + next dispatch | local checks | advisory on incomplete claim |

## Tradeoffs
- +1 brain.think RPC round-trip per intent turn (~50-100ms); fail-soft to census on daemon-down.
- Loophole writes only on fire (narrow); age-out keeps reinject lean.
- Verifier advisory (no FP block cost) per user decision.

## Out of scope
- The ~13k-token harness TaskList is Claude Code's own system, NOT kavach — cannot narrow from here.
- No re-adding embeddings (decision/onnx-removal stands).
