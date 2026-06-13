# Loop Engineering → Kavach Injection Strategy

**Date:** 2026-06-10
**Project:** kavach-rs
**Author:** research-director + Explore synthesis (read-only investigation)
**Status:** RESEARCH — decision pending owner direction

---

## TL;DR

**Loop engineering** is the discipline that comes *after* harness engineering: you stop prompting the
agent and instead build the system that prompts it — on a schedule, against a goal, with a verification
gate between iterations and persistent memory outside the context window. **Kavach already *is* a
loop-engineering harness** (intent classification → kanban dispatch → stop-gate `[AUTO_CONTINUE]` →
3-witness verify). The frontier work — **OpenClaw**, **Hermes Agent**, the **agentic-RL** literature,
and the **agent-memory canon** (Reflexion / Voyager / ExpeL / Generative Agents) — converges on one
pattern Kavach implements only *partially*: a **closed self-improving loop** where every run's outcome
is compressed into reusable memory and *injected back* into the next run's prompt, relevance-gated.

The gap is not the loop. The gap is **what gets injected, when, and how relevance-gated it is.** Today
Kavach injects classification (`[INTENT]`, `[HARNESS]`, `[PHASE]`) and a frequency-top-5 pattern dump
(`[SELF_EVOLVE_PATTERNS]`). To "behave in the loop-engineering direction" it should inject **five new
context frames**, each backed by an existing DB store:

| Frame | Embodies | Backed by (exists today) | Gate |
|---|---|---|---|
| `[LOOP]` | loop engineering / the `/goal` primitive | `roadmap.harness`, `[PHASE]` | intent + stop |
| `[REWARD]` | RLVR + GRPO credit assignment | `reward_backfill`, 3-witness verify | stop + session_start |
| `[MISTAKE_GUARD]` | Reflexion verbal RL (negative examples) | `entity(mistake_event)` + embeddings | pre_write |
| `[SKILL]` | Voyager / Hermes procedural memory | `pattern` rows + `SKILL.md` + pattern-extractor | intent + pre_write |
| `[CONCEPT]` | new-concept awareness | `entity(L0 concept)` + `[SELF_EVOLVE] novel_error` | session_start |

The single highest-leverage change: **replace frequency-ranked injection with relevance-gated retrieval**
(Generative-Agents scoring: recency × relevance × importance; ERL top-k), and **inject at the point of
action** (pre_write) rather than only at session start.

---

# Part I — The Landscape

## 1.1 Loop engineering (the new layer)

> "Loop engineering is building a system that prompts your agent on a schedule and against a goal,
> instead of typing each prompt yourself." — popularized by Addy Osmani, echoing Peter Steinberger and
> Claude Code lead Boris Cherny ("my job is now to write loops").

Three-generation maturity model (2026 consensus):

```
Prompt engineering (2022-24)  →  Context engineering (2025)  →  Harness engineering (2026)  →  Loop engineering
  craft one instruction          assemble the context window     design the execution env       design the system that
                                                                  (tools, constraints, gates)    prompts the agent on a loop
```

**Harness vs loop (the clean distinction):** a *harness* runs a single agent's execution environment;
a *loop* coordinates agents (and sub-agents) across **scheduled cycles with verification gates**. Loop
engineering's signature primitive is **`/goal`** — "the most discussed agent primitive of 2026" —
because it lets the loop decide it is *finished* without a human in the seat. Termination is defended in
depth: a goal predicate, max iterations, wall-clock timeout, cost budget, and loop-detection
fingerprinting of repeated states.

**Why it matters for Kavach:** Kavach's stop-gate `[AUTO_CONTINUE]` *is* a `/goal` loop. But the model
on the receiving end is not told the **loop invariant** — what the goal is, which iteration it is on,
and what predicate ends the loop. It is told "do not stop." Loop engineering says: make the loop frame
*legible to the agent*, so it self-terminates correctly instead of being force-blocked.

Sources: Loop Engineering Guide (lushbinary.com), The Anatomy of an Agent Loop (Steve Kinney),
Anthropic *Harness Design for Long-Running Apps*.

## 1.2 Harness engineering (the floor below)

Canonical thesis (Mitchell Hashimoto / OpenAI Codex): **"Agents aren't hard; the harness is hard."**
`Agent = Model + Harness`. Core principles directly relevant to Kavach:

- **Constraints via linters, not prompts.** Kavach's `clippy -D warnings`, `unsafe forbidden`,
  `dead_code denied` baseline is exactly this — a *feedforward constraint harness*.
- **Generator ≠ Evaluator.** Anthropic found models **cannot reliably evaluate their own work**
  (systematic leniency). Separate the maker from the checker. Kavach's **3-witness verify** (rg +
  `git diff --stat` + `cargo check`) is an *external* evaluator — correctly NOT self-assessed.
- **Context resets beat compaction.** Clearing with a structured handoff outperformed in-place
  summarization; Opus exhibited "context anxiety" near limits. Kavach's `session_runtime` INI blob
  keyed by `session_id` (fresh state on `/clear`) is the reset substrate.
- **Every harness component encodes an assumption about what the model can't do — and those go stale as
  models improve.** This is the maintenance burden Kavach must budget for: injected directives are
  assumptions with a TTL.

## 1.3 OpenClaw (github.com/openclaw/openclaw)

A local-first personal-assistant harness. Two ideas transfer to Kavach:

1. **Injected prompt-file trinity** — `AGENTS.md` (instructions), `SOUL.md` (persona/behavioral spine),
   `TOOLS.md` (capability reference). This is the *static* analogue of Kavach's *dynamic* gate
   injection. Kavach's gates are strictly more powerful (DB-backed, context-aware, per-turn) — but the
   **"soul" idea is missing**: a stable behavioral spine reinforced every session, distinct from the
   volatile per-turn context. (Kavach's `CLAUDE.md` directives are the de-facto soul, but they are not
   re-asserted by a gate; they rely on the host reading the file.)
2. **Skills as procedural memory** — `~/.openclaw/workspace/skills/<skill>/SKILL.md`, Markdown +
   YAML frontmatter, managed by a registry (ClawHub). Identical in spirit to Kavach's `SKILL.md`
   skills and `pattern` store.

There is also **OpenClaw-RL** ("train any agent simply by talking") and **awesome-openclaw-agents**
(162 templates) — evidence that the skills-as-files model scales to a community registry.

## 1.4 Hermes Agent (Nous Research) — the self-improving-loop blueprint

This is the **most directly relevant external system**, and it is what Kavach's `pattern-extractor`
agent is already described as ("Hermes-shaped procedural-memory extractor"). Five pillars:

| Pillar | Mechanism | Kavach analogue |
|---|---|---|
| **Memory** | `user.md` + `memory.md`, FTS5 search + LLM summarization, evolving user model | `decision`/`research` rows; `session_runtime`; the DB *is* memory.md |
| **Skills** | Markdown+YAML procedural memory, 91 built-in + 520 community, **progressive disclosure** (load only relevant) | `SKILL.md` + `pattern` store; **progressive disclosure ≈ the missing relevance gate** |
| **Soul** | `soul.md` tone/behavior, multiple instances share a model with distinct registers | `CLAUDE.md` directives (un-reasserted) |
| **Crons** | natural-language scheduling, isolated sessions prevent recursive job creation | the autonomous loop / stop-gate dispatch |
| **Self-Improving Loop** | *emergent* — work → learn → memory/skill update → reference past → "gets more capable the longer it runs" | `gate_pattern` promotion + mistake ledger (partial) |

**The killer mechanism:** after completing a complex multi-tool task, Hermes **autonomously writes a
skill** — a markdown doc capturing the procedure, pitfalls, verification steps, and required env. Next
time a similar task appears, it **loads the skill instead of reasoning from scratch**, and the skill
**self-improves** when a better approach is found.

**Kavach has every piece to do this and does not yet close it:** the `pattern-extractor` agent exists,
the `pattern` store exists, the 3-witness verify provides the "task succeeded" trigger — but there is no
**"on verified completion → extract skill → store → inject next time"** edge wired end-to-end.

## 1.5 RL applied to the loop (not the weights)

The agentic-RL literature gives Kavach a vocabulary for what it already half-does:

- **RLVR (RL with Verifiable Rewards):** reward = did the artifact pass the test? `1.0` / `0.0`. This is
  *exactly* Kavach's 3-witness verify. Kavach already calls `reward_backfill::backfill_session_rewards`.
- **GRPO credit assignment:** advantage = `(R_i − mean(R)) / std(R)` over a *group* of rollouts — no
  value network needed. The orchestration-time analogue: score an action's reward **relative to the
  session/recent baseline**, and tell the model whether it is *above or below* baseline. That is a
  cheap, legible credit signal Kavach can inject.
- **Reward shapes:** binary (incorruptible, sparse) vs composite (dense, needs tuning) vs staged
  (milestone gradient) vs length-penalized. Kavach's verify is binary today; a **composite** reward
  (compiles + tests pass + no new clippy + diff landed) is a natural upgrade.
- **Verifier-gated loops / AgentV-RL:** the verifier is itself a multi-turn tool-using agent. Kavach's
  stop-gate pipeline is a deterministic verifier; it could escalate to an agentic verifier for
  ambiguous "is this really done" calls.
- **Exploration vs exploitation:** when to *reuse* a learned pattern (exploit) vs *try something new*
  (explore). Kavach's `gate_pattern` promotion-at-50 is a pure-exploitation frequency rule; it has no
  explore signal.

**Caveat from the research:** native agentic RL bakes behavior *into weights*. Kavach operates at
**inference/orchestration time** — it cannot update weights. So Kavach's "RL" is **verbal/experiential
RL** (Reflexion-style): the reward and the lesson are stored as *text and embeddings* and re-injected,
not back-propagated. This is the correct and only available lever, and it is exactly what the memory
canon below formalizes.

## 1.6 The agent-memory canon (how memory feeds the next loop)

| System | Mechanism | What Kavach should steal |
|---|---|---|
| **Reflexion** (NeurIPS'23) | verbal RL: reflect on failure in NL, store in episodic buffer, no weight update | inject the reflection as a **negative example before the next attempt** |
| **Voyager** (2023) | ever-growing **skill library** of verified executable code, retrieved+composed | the auto-extract-on-success → retrieve-on-similar edge |
| **ExpeL** (AAAI'24) | extract NL insights from trajectories; **flaw: concatenates *all* insights** regardless of relevance | the flaw is Kavach's current `[SELF_EVOLVE_PATTERNS]` top-5 dump |
| **ERL** (ICLR'26) | **selective top-k** relevance retrieval of heuristics with explicit trigger conditions; +5.2% over ExpeL | **the fix**: relevance-gated injection, not frequency dump |
| **Generative Agents** (UIST'23) | memory stream scored by **recency × relevance × importance**; reflection synthesizes higher-level notions | the retrieval scoring function for *what to inject* |
| **Experience Compression Spectrum** (2026) | unify memory/skills/rules on one axis: L0 raw → L1 episodic (5-20×) → L2 procedural (50-500×) → L3 declarative (1000×+); **open problem: no system does adaptive cross-level compression** | Kavach's L0→L3 entity graph maps onto this *exactly* — an opportunity to lead |

The single empirical headline: **skill systems beat raw memory retrieval by a wide margin** (SkillRL
reports **+68.5 points** over trajectory retrieval on ALFWorld). Procedural memory (L2) > episodic dump
(L1). Kavach should bias toward **distilling patterns/skills**, not accumulating raw decision rows.

---

# Part II — Kavach Today (evidence-grounded)

All paths under `/Users/gauravwankhede/kavach-rs/`.

## 2.1 The injection substrate

Every Claude Code lifecycle event runs a gate that builds a context string and emits it as
`additional_context` JSON on stdout:

- `crates/kavach-hook/src/lifecycle.rs:17` — `exit_user_prompt_submit(context)` → the canonical
  injection point (writes `UserPromptSubmitOutput { additional_context }`).
- `crates/kavach-hook/src/lifecycle.rs` — `exit_stop_block(...)` → the Stop-hook injection that forces
  continuation.

Six hooks bracket a session: `SessionStart` (`gates/session_start.rs:30`), `UserPromptSubmit`
(`gates/intent.rs:32`), `PreToolUse`/`PostToolUse` (`gates/pre_*`, `gates/post_*`), `Stop`
(`gates/stop.rs:40`), `SessionEnd` (`gates/session_end.rs`).

## 2.2 What is injected today

- **`[INTENT]`** — `gates/intent/kvs.rs:4` builds `type/confidence/complexity/risk` + `search_year/
  month/week`. Classification is LLM-backed via `kavach_chain::analyze_intent` (`gates/intent.rs:50`).
- **`[HARNESS]`** — `gates/intent/harness.rs:24` keyword-classifies the prompt into one of six patterns
  (`classify-act`, `fan-out-synthesize`, `worker-critic`, `generate-filter`, `pairwise-tournament`,
  `loop-until-done` default) and **persists it onto the next-open roadmap card** via RPC
  `db.set_harness` (`harness.rs:~90`).
- **`[PHASE]`** — `current / iteration / files_done`.
- **`[RAG:skill]`** — top-1 skill name (`gates/intent/context.rs`).
- **`[AUTO_CONTINUE]` / `STOP BLOCKED`** — `gates/stop/dispatch/first_pass/task.rs` claims the next card
  (flips `todo → in_progress`) and hard-blocks the stop; `gates/stop/dispatch/retry/reblock.rs` emits
  the tiered `STOP BLOCKED (n/max)` re-block.
- **`[SELF_EVOLVE_PATTERNS]`** — `gates/session_start/patterns.rs:13` calls RPC `gate_pattern.list_hot`
  (limit 5) and dumps the top patterns by `occurrence_count`. **This is the ExpeL flaw in production:
  frequency-ranked, not relevance-gated, injected once at session start.**

## 2.3 The persistent stores (SurrealDB, single-writer RPC)

`crates/kavach-surreal/src/schema.rs:18` defines the typed tables: `decision`, `research`, `pattern`,
`roadmap`, `app_spec`, `session_runtime`, `entity`, `event`, `gate_pattern`.

- **`gate_pattern`** (`gate_patterns.rs:31`) — `error_tokens`, `fix_strategy`, `imperative_rewrite`,
  `dsa_rationale`, `occurrence_count`, `bloom_bytes`, `tier`. Promotion threshold
  `PROMOTION_THRESHOLD = 50` (`gate_patterns.rs:14`): at 50 occurrences a row is promoted `research →
  autonomous` and a **bloom filter** is built. Matching uses **TF-IDF cosine** with `MIN_SIM = 0.35`
  (`gate_patterns.rs:26`), bloom-filter quick-reject first.
- **`entity`** — knowledge-graph nodes. Mistakes are appended as `entity_type = 'mistake_event'` with a
  384-dim embedding (`graph/mistakes/append.rs:19`); schema indexes it HNSW/COSINE
  (`schema.rs:270`). L0 concepts also live here.
- **`event`** — append-only audit trail (the L0 raw-trace tier).
- **`session_runtime`** — durable INI state blob keyed by `session_id`.

## 2.4 The closed loop that already works

```
WRITE (during session, stop gate):
  gates/stop.rs:83  detect_unpersisted_decision() → record_mistake() → entity(mistake_event)+embedding
  gate_pattern.upsert()  → occurrence_count++ ; promote to autonomous at 50 ; build bloom
  reward_backfill::backfill_session_rewards()   ← reward signal already computed

READ (next session start):
  gates/session_start/patterns.rs:13  gate_pattern.list_hot(5) → [SELF_EVOLVE_PATTERNS] → injected
```

This is a real self-improving loop. Its two weaknesses: (1) the **read side is frequency-ranked, not
relevance-gated**, and (2) it injects **only at session start**, never **at the point of action**.

## 2.5 Honest gaps (from the Explore map)

- No fully-wired **anti-pattern clustering (L3)** read path — mistakes are embedded (L2) but clustered
  centroids are not surfaced into injection.
- No **skill auto-extraction on verified completion** (the Hermes/Voyager edge).
- No **reward injection** — `reward_backfill` computes a signal the model never sees.
- No **new-concept ingestion** beyond the `[SELF_EVOLVE] novel_error` path (which fires for *tool
  errors* only, not for *concepts learned from research*).
- The `~0.85` similarity threshold referenced in `CLAUDE.md` is the **title-dedup gate** on
  `db write --new` (confirmed: "refuses if similarity ≥ 0.85"), **not** a memory-retrieval threshold.
  Retrieval uses `MIN_SIM = 0.35` (TF-IDF) and HNSW cosine. (Documentation drift worth fixing.)

---

# Part III — Gap Analysis

| Loop-engineering / self-improving capability | Frontier reference | Kavach status | Gap |
|---|---|---|---|
| Goal-legible loop frame for the agent | `/goal` primitive | `[AUTO_CONTINUE]` blocks stop, hides the invariant | **inject `[LOOP]`** |
| Reward visible to the agent | RLVR / GRPO | `reward_backfill` computed, never injected | **inject `[REWARD]`** |
| Negative examples at point of action | Reflexion | mistakes stored, injected only at session start | **inject `[MISTAKE_GUARD]` in pre_write** |
| Procedural memory auto-extracted on success | Voyager / Hermes | `pattern-extractor` exists, edge not wired | **wire extract-on-verify → `[SKILL]`** |
| Relevance-gated retrieval (top-k) | ERL / Generative Agents | frequency top-5 dump | **swap ranking to recency×relevance×importance** |
| New-concept ingestion + propagation | — | only `novel_error` for tools | **inject `[CONCEPT]`, ingest research → L0** |
| Adaptive cross-level compression L0→L3 | Experience Compression Spectrum (open problem) | tiers exist, transitions manual | **opportunity to lead** |

---

# Part IV — The Injection Design (the core ask)

Five frames. Each marked **ENHANCE** (existing mechanism) or **NEW**. Each shows the **gate**, the
**backing store**, and **example injected text** (the literal `additional_context` the model receives).

## 4.1 `[LOOP]` — make the loop invariant legible · ENHANCE

**Gate:** `UserPromptSubmit` (intent) emits the frame; `Stop` refreshes `iteration`.
**Backing:** `roadmap.harness`, `[PHASE]` fields, the `/goal` predicate.
**Why:** loop engineering says the agent should be able to *self-terminate against the goal*. Replace
the bare "do not stop" with the full invariant so the model converges instead of being force-blocked.

```
[LOOP]
goal: <card.title — the verifiable objective>
harness: loop-until-done
iteration: 3
termination: 3-witness verify passes (rg artifact ∧ git diff --stat ∧ cargo check exit 0)
continue_if: any witness missing
budget: <tokens remaining> | max_iter: 25
on_done: write card → verified, then dispatch next (same turn)
```

This turns Kavach's force-block into a **goal-conditioned loop the model understands**, which is the
defining move of loop engineering.

## 4.2 `[REWARD]` — surface the RL signal · NEW (data already exists)

**Gate:** `Stop` (after verify) and `SessionStart` (running stats).
**Backing:** `reward_backfill::backfill_session_rewards` (already computed), 3-witness verify outcome.
**Why:** RLVR + GRPO. The model behaves RL-natively only if it *sees* credit assignment. Use a
**group-relative advantage** (GRPO) against the recent baseline — cheap and legible.

```
[REWARD]
last_action: edit gates/stop.rs → verify PASSED (+1.0)
session_pass_rate: 0.78 (7/9)
baseline_30d: 0.61
advantage: +0.17 above baseline — this approach is working; exploit it
signal: composite (compiles ∧ tests ∧ no_new_clippy ∧ diff_landed)
```

Upgrade path: binary reward → **composite reward** (the four-factor signal above), which is denser and
matches the harness-engineering "constrain the solution space" principle.

## 4.3 `[MISTAKE_GUARD]` — Reflexion at the point of action · ENHANCE

**Gate:** `PreToolUse` on `Write|Edit` (NOT just session start) — inject the lesson *before the keystroke
that would repeat it*.
**Backing:** `entity(mistake_event)` + 384-dim embeddings (`graph/mistakes/append.rs:19`).
**Why:** Reflexion verbal RL. Embed the *pending edit's intent*, retrieve the **top-k relevant**
mistakes by cosine similarity (ERL selective retrieval), inject as negative examples.

```
[MISTAKE_GUARD] (2 relevant priors, sim > 0.45)
✗ banned: "settled a decision in prose without persisting it"
  ✓ correct: write a decision row the same turn
✗ banned: "marked card done without cargo check"
  ✓ correct: run 3-witness verify before status-update
```

This is the **single highest-value relocation**: same data, injected at the moment of risk instead of
at session start where it has decayed from attention.

## 4.4 `[SKILL]` — Voyager/Hermes procedural memory · NEW EDGE (parts exist)

**Two halves:**

1. **Extract-on-verify (write):** in the `Stop` gate, when a multi-step card passes 3-witness verify,
   dispatch the `pattern-extractor` (Hermes-shaped) agent to distill a `SKILL.md`-shaped `pattern` row:
   procedure + pitfalls + verify steps + required env. (The agent and the `pattern` store both exist;
   only the trigger edge is missing.)
2. **Retrieve-on-similar (read):** in the `intent`/`pre_write` gate, embed the task, retrieve the
   best-matching skill, inject it so the model **loads the procedure instead of re-deriving it**.

```
[SKILL] matched "add a new gate" (sim 0.71, used 4×, success 4/4)
1. add gates/<name>.rs with run(&HookInput) -> Result<(), EngineError>
2. register in the gate dispatch table (gates/mod.rs)
3. build context via kavach_hook::context_block("<TAG>", &kvs)
4. emit via exit_* ; verify: cargo check ∧ nextest run -p kavach-engine
pitfall: forgetting the dispatch registration compiles but never fires
```

Procedural memory (L2) is the highest-ROI memory tier per the research (+68.5 pts evidence). This edge
is what makes Kavach "get more capable the longer it runs."

## 4.5 `[CONCEPT]` — new-concept awareness · NEW

**Gate:** `SessionStart` (recently-learned concepts) + `intent` (concept relevant to *this* prompt).
**Backing:** `entity(entity_type='concept')` (L0), fed by the `research` store and the existing
`[SELF_EVOLVE] novel_error` novelty detector — generalized from *tool errors* to *concepts*.
**Why:** the user's explicit "new concepts awareness." When research surfaces a concept the DB has
never embedded (e.g. *loop engineering* today), ingest it as an L0 concept and propagate it so the model
*applies* fresh knowledge instead of operating on its training cutoff.

```
[CONCEPT] recently learned (L0, last 30d)
• loop-engineering: design the system that prompts the agent on a schedule vs a goal,
  with a verify gate between iterations — apply when scoping autonomous work.
• GRPO: group-relative advantage, no value net — use for the [REWARD] baseline math.
source: docs/loop-engineering-injection-strategy.md
```

Novelty test reuses the machinery already in place: an embedding with no near neighbor (cosine <
threshold) in `entity` = novel → write L0 concept → inject. This directly closes the model's
**knowledge-cutoff gap** (today: 2025-01) using the DB as a live concept feed.

## 4.6 Injection ordering & token budget

Order by *decay risk* — most-perishable closest to the action:

```
SessionStart : [CONCEPT] [REWARD:stats] [SELF_EVOLVE_PATTERNS→relevance-gated] [SOUL re-assert]
UserPrompt   : [INTENT] [HARNESS] [LOOP] [SKILL:task-match] [CONCEPT:prompt-match]
PreWrite     : [MISTAKE_GUARD:top-k] [SKILL:step-match]      ← point-of-action, highest value
Stop         : [REWARD:last-action] [LOOP:iteration++] [AUTO_CONTINUE | clean-exit]
```

Token discipline (global directive §7): every frame is **relevance-gated and capped** (top-k, k≤3;
similarity floor). Frequency dumps are banned — they are the ExpeL anti-pattern. An empty retrieval
injects **nothing**, not a placeholder.

---

# Part V — Persistent Memory as a Compression Spectrum

Map Kavach's stores onto the 2026 Experience-Compression axis and make the **tier explicit at write
time** so the model knows *where* a finding belongs:

| Tier | Compression | Kavach store | Written when |
|---|---|---|---|
| **L0 raw** | 1:1 | `event` (audit), `session_runtime` | every action |
| **L1 episodic** | 5–20× | `decision`, `research` | a fact/choice settled this turn |
| **L2 procedural** | 50–500× | `pattern`, `gate_pattern`, `SKILL.md` | a *repeatable procedure* verified |
| **L3 declarative** | 1000×+ | clustered anti-patterns, `CLAUDE.md` invariants | a cause recurs across cards/projects |

**Adaptive cross-level compression** (the research's stated open problem) is Kavach's chance to lead:
a maintenance job that **promotes** memory up the spectrum on evidence —
N similar L1 decisions → distill one L2 pattern; M similar L2 patterns/mistakes → cluster into one L3
anti-pattern (the HNSW centroid already half-built in `graph/mistakes/`). Inject only the **highest
tier that applies** (an L3 invariant beats five L1 rows at 1/200th the tokens).

**Retrieval scoring (replace frequency-top-5):** Generative-Agents formula —
`score = w_r·recency + w_s·cosine_relevance + w_i·importance` — pick top-k above a floor. This is the
one change that fixes the ExpeL flaw now in production at `gates/session_start/patterns.rs`.

---

# Part VI — RL-Awareness Layer

Kavach cannot update weights; it does **verbal/experiential RL**. The mapping:

| RL concept | Kavach realization | Status |
|---|---|---|
| Verifiable reward (RLVR) | 3-witness verify → 0/1, upgraded to composite 4-factor | reward exists, **inject it** |
| Credit assignment (GRPO) | advantage = action_reward − session_baseline, injected in `[REWARD]` | NEW |
| Policy | `gate_pattern` learned fixes (the action→fix map) | exists |
| Negative examples | `mistake_event` ledger (banned_sample + correct_action) | exists, **relocate to pre_write** |
| Experience replay | re-inject past verified trajectories as `[SKILL]` | NEW edge |
| Exploration vs exploitation | promotion-at-50 is pure exploit; add an **explore signal** | NEW |

**Explore/exploit, concretely:** when `gate_pattern` confidence is high (autonomous tier, many
successes) inject "exploit: reuse this fix." When a pattern is *stale* (model version changed — recall
harness assumptions go stale) or low-confidence, inject "explore: the prior fix is unverified on this
model; try fresh and record the outcome." This keeps the policy from over-fitting to a frozen model
generation.

---

# Part VII — New-Concept Awareness (closing the cutoff gap)

The model's knowledge cutoff is `2025-01`; the calendar is `2026-06`. The DB is the bridge.

```
detect:   research finding / external doc → embed → no near-neighbor in entity(concept)?  → NOVEL
ingest:   write entity(entity_type='concept', name, embedding, source, one_line)          → L0
propagate: SessionStart injects [CONCEPT] recently-learned; intent injects prompt-relevant concepts
reinforce: when a concept is *used* in a verified task, bump its importance (Generative-Agents weight)
```

This reuses the **exact machinery** of `[SELF_EVOLVE] novel_error` (which already detects unseen *tool
errors* via fingerprint and asks for a stored fix) — generalized from errors to concepts. **Meta-proof:
this very report is the first L0 concept to ingest** — "loop engineering" did not exist at the model's
cutoff, and now it lives in the DB and is injected forward.

---

# Part VIII — Phased Roadmap & What to Persist Now

**Phase 1 (highest leverage, lowest risk):**
1. Relocate mistake injection to `pre_write` as `[MISTAKE_GUARD]`, relevance-gated top-k (reuses
   existing embeddings).
2. Swap `[SELF_EVOLVE_PATTERNS]` ranking from `occurrence_count` to recency×relevance×importance.
3. Inject `[REWARD]` from the already-computed `reward_backfill`.

**Phase 2 (the self-improving edge):**
4. Wire extract-on-verify → `pattern-extractor` → `[SKILL]` retrieval (the Hermes/Voyager loop).
5. Replace `[AUTO_CONTINUE]`'s "do not stop" with the goal-legible `[LOOP]` frame.

**Phase 3 (frontier):**
6. `[CONCEPT]` ingestion + injection (generalize `novel_error`).
7. Adaptive L0→L3 compression maintenance job (lead on the open problem).

**Persist this turn (global directive §sync_to_kavach_db):**
- `research` row: this report (key `loop-engineering-injection-2026`) so the next session reuses it.
- `entity(concept)` L0: `loop-engineering`, `harness-engineering`, `GRPO`, `RLVR`,
  `experience-compression-spectrum` — the new concepts to propagate.
- `roadmap` cards: the seven phased items above, dependency-ordered (Phase 2 depends-on Phase 1).

---

## Sources

**Loop / harness engineering:** Anthropic *Harness Design for Long-Running Apps*
(anthropic.com/engineering); Loop Engineering Guide (lushbinary.com); The Anatomy of an Agent Loop
(stevekinney.com); The Third Evolution — Harness Engineering (epsilla.com); Augment Code Harness
Engineering guide.
**OpenClaw:** github.com/openclaw/openclaw (AGENTS.md / SOUL.md / TOOLS.md, skills/); OpenClaw-RL
(Gen-Verse); awesome-openclaw-agents.
**Hermes Agent:** hermes-agent.nousresearch.com/docs; MindStudio 5-Pillar Architecture; aiagentmemory.org
Hermes Agent Memory; NousResearch/Hermes-Function-Calling.
**Agentic RL:** Inside the Agentic RL Training Loop (blog.guanghan.ai); AgentV-RL (arXiv 2604.16004);
Open-AgentRL (Gen-Verse, ICML'26); Apple ML — RL for Long-Horizon Agents.
**Memory canon:** Reflexion (arXiv 2303.11366); Voyager (voyager.minedojo.org); ExpeL (arXiv 2308.10144);
Generative Agents (UIST'23); ERL (arXiv 2603.24639, ICLR'26); Experience Compression Spectrum
(arXiv 2604.15877); Self-Improving AI Agents 2026 Guide (o-mega.ai).

**Kavach internals (file:line):** `crates/kavach-hook/src/lifecycle.rs:17`;
`crates/kavach-engine/src/gates/intent.rs:32,50`; `gates/intent/kvs.rs:4`; `gates/intent/harness.rs:24`;
`gates/stop.rs:40,83`; `gates/stop/dispatch/first_pass.rs:20`; `gates/session_start.rs:30`;
`gates/session_start/patterns.rs:13`; `crates/kavach-surreal/src/schema.rs:18,270`;
`crates/kavach-surreal/src/gate_patterns.rs:14,26,31,145,207`;
`crates/kavach-surreal/src/graph/mistakes/append.rs:19`.
