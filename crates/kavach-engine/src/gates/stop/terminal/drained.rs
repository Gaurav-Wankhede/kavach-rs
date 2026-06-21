//! Shared drained-board terminal verdict — the SINGLE source of truth both stop
//! terminals emit when the dispatch tiers find no runnable card.
//!
//! Three states hide behind "nothing dispatchable" with DIFFERENT outcomes. None
//! tells the LLM to stop: the loop runs until the user halts it with `Esc`.
//!
//! 0. The session is pinned to a lane (`KAVACH_LANE`) and its lane + the unlaned
//!    backlog are both drained → `[LANE_DRAINED]` (lane.rs). Never cross into a
//!    foreign lane; that is another session's work.
//! 1. The board still holds runnable-status cards, but EVERY one is held back by
//!    an unmet dependency → `[ALL_BLOCKED]`: re-scan the DB, name the blockers.
//! 2. No runnable-status card. Re-scan roadmap + decisions (ALL statuses) and the
//!    active `[PLAN]` for the next actionable item → `[AUTO_CONTINUE]`.
//!
//! Lives HERE (`pub(in crate::gates::stop)`) so BOTH the first-pass terminal
//! (`clean_exit`) and the retry terminal emit the IDENTICAL verdict. The verdict
//! is loop-SAFE: callers emit it via `exit_stop_context` (allows the turn to end,
//! no hard block), so it can never spin.

mod lane;

/// The census-aware terminal context for a drained dispatch.
///
/// `open_set_census` returns `Some((runnable, blocked))` or `None` on RPC outage;
/// `None` fails closed to the nudge (never a wrong clean-stop on an unobservable
/// board).
pub(in crate::gates::stop) fn drained_terminal_context(project: &str) -> String {
    if let Some(lane_name) = lane::lane_env() {
        return lane::lane_drained_context(&lane_name);
    }
    let census = crate::gates::stop_dispatch::open_set_census(project);
    // A dependency cycle is NOT a legitimate block: it is a deadlock the AI must
    // repair (break the cycle), never a clean stop. Surface it before any
    // all-blocked / plan verdict so it cannot forge a false `[ALL_BLOCKED]`.
    if census.is_some_and(|(_, _, cyclic)| cyclic > 0) {
        return cycle_deadlock_context();
    }
    if census_is_all_blocked(census) {
        all_blocked_context(census)
    } else {
        board_drained_plan_context(census)
    }
}

/// One-line census STAMP proving the gate read the kavach DB roadmap table THIS
/// stop. `Some` → the live counts; `None` (RPC outage) → an explicit
/// unobservable marker so the verdict never *claims* a read it could not make.
/// This is the leaf-evidence the drained verdict must carry (`verdict_needs_leaf_evidence`).
fn census_stamp(census: Option<(u64, u64, u64)>) -> String {
    match census {
        Some((runnable, blocked, cyclic)) => format!(
            "gate read the kavach DB roadmap table this stop -> \
             runnable={runnable} blocked={blocked} cyclic={cyclic} \
             (no dispatchable card remained, so the gate let the turn end)"
        ),
        None => "kavach DB roadmap table was UNOBSERVABLE this stop (RPC outage) -> \
                 the gate could not confirm the board; treat the backlog as non-empty"
            .to_owned(),
    }
}

/// True iff the census proves a DISPATCHABLE remainder the single-source dispatch
/// probe failed to surface: at least one runnable-status card AND not every one of
/// them is blocked/cyclic — i.e. a card the gate counts as runnable+unblocked yet
/// `next_dispatch` returned None (census reads roadmap-by-status + the `TaskList`
/// fold; dispatch reads only the roadmap table with lane/umbrella filters). This is
/// the census/dispatch divergence: the loop must NOT clean-stop on it — runnable
/// roadmap todos remain. `None` (RPC outage) → false (the outage path already fails
/// closed to the non-empty nudge elsewhere; do not double-refuse here).
const fn census_has_dispatchable_remainder(census: Option<(u64, u64, u64)>) -> bool {
    match census {
        Some((runnable, blocked, cyclic)) => {
            runnable > 0 && blocked.saturating_add(cyclic) < runnable
        }
        None => false,
    }
}

/// Public divergence check for the terminal callers: did the census prove a
/// dispatchable roadmap todo the probe missed? When true the caller REFUSES the
/// stop (parity with the loophole / cycle refuse-stops) so the loop always acts on
/// remaining roadmap todos instead of clean-stopping on a single-source None.
#[must_use]
pub(in crate::gates::stop) fn roadmap_todos_remain(project: &str) -> bool {
    census_has_dispatchable_remainder(crate::gates::stop_dispatch::open_set_census(project))
}

/// The REFUSE-STOP verdict emitted when `roadmap_todos_remain` is true: command a
/// DIRECT roadmap-todo query (not the dispatch probe that just missed them) and a
/// claim+start THIS turn. This makes "always check the roadmap for todos" an
/// ENFORCED action, not advisory prose.
pub(in crate::gates::stop) fn roadmap_todos_remain_context(project: &str) -> String {
    let stamp = census_stamp(crate::gates::stop_dispatch::open_set_census(project));
    let block = kavach_hook::context_block(
        "ROADMAP_TODOS_REMAIN",
        &[
            ("census", &stamp),
            (
                "why",
                "the census proves runnable, UNBLOCKED roadmap todos remain, but the \
                 dispatch probe (lane-affinity + umbrella filter, roadmap table only) \
                 returned nothing — a census/dispatch source divergence. Runnable work \
                 exists; this is NOT a drained board and NOT a clean stop.",
            ),
            (
                "action",
                "Do NOT stop. Query the roadmap DIRECTLY for the remaining todos the \
                 probe missed: `kavach db query-raw --query \"SELECT meta::id(id) AS k, \
                 title FROM roadmap WHERE entry_status IN ['todo','in_progress']\"` (and \
                 `--category decision` for unexecuted rows). Pick the highest-priority \
                 todo, claim it (`kavach db status-update --status in_progress`), and \
                 START it THIS turn. Yield only to the user's `Esc`.",
            ),
        ],
    );
    format!("{block}{RESEARCH_MODE_DIRECTIVE}")
}

/// True iff the census proves a BLOCKED remainder: at least one runnable-status
/// card AND every one of them blocked by dependencies or cyclic. `None` (RPC
/// outage) → false → fail closed to the PLAN nudge. An empty board (`runnable
/// == 0`) is NOT all-blocked. A cycle is handled BEFORE this by
/// `drained_terminal_context`, so here `blocked + cyclic == runnable` still
/// counts as the all-blocked remainder.
const fn census_is_all_blocked(census: Option<(u64, u64, u64)>) -> bool {
    match census {
        Some((runnable, blocked, cyclic)) => {
            runnable > 0 && blocked.saturating_add(cyclic) == runnable
        }
        None => false,
    }
}

/// A dependency cycle holds runnable cards hostage — no card in the cycle can ever
/// become ready, so the loop would otherwise spin or falsely clean-stop. This is
/// AI-repairable work (break the cycle), so REFUSE the stop and direct the fix.
fn cycle_deadlock_context() -> String {
    kavach_hook::context_block(
        "CYCLE_DEADLOCK",
        &[
            (
                "why",
                "one or more runnable cards declare a dependency CYCLE (a card depends \
                 on itself, or A->B->A). No card in a cycle can ever satisfy its deps, \
                 so it is permanently un-dispatchable — this is a deadlock, NOT a \
                 legitimate block and NOT a clean stop.",
            ),
            (
                "action",
                "Do NOT stop. Run `kavach db kanban --format mermaid` to see the cycle, \
                 then break it: edit the offending card's `DEPENDS_ON:`/`BLOCKED_BY:` \
                 line to remove the back-edge (or re-order the work). Re-verify the \
                 census has zero cyclic cards before stopping.",
            ),
        ],
    )
}

/// Case 1: every runnable card is dependency-blocked. NOT a stop, NOT a hand-off:
/// WALK to the blocker, BUILD it leaf-first; a missing credential gets FILED as a
/// card while the loop keeps draining. Yields only to `Esc`.
fn all_blocked_context(census: Option<(u64, u64, u64)>) -> String {
    let stamp = census_stamp(census);
    let block = kavach_hook::context_block(
        "ALL_BLOCKED",
        &[
            ("census", &stamp),
            (
                "action",
                "Do NOT stop. Gate ALREADY read the DB this stop (`census` above) — do \
                 NOT re-run `kavach db kanban`. RESOLVE-AND-CONTINUE, dependency-first: \
                 (1) For each card, read its `DEPENDS_ON:`/`BLOCKED_BY:` line and WALK \
                 to the blocking card (`kavach db get --category roadmap --key \
                 <blocker>`). (2) RESEARCH the ACTUAL conflict (WebSearch the current \
                 authoritative source). (3) BUILD the blocker THIS turn — recurse to \
                 ITS blocker until a leaf dispatches, then dispatch the dependent. \
                 (4) If the dependency edge is STALE/FALSE (prerequisite already \
                 shipped, or never applied): correct the `DEPENDS_ON:` line (`kavach db \
                 write`) and dispatch. (5) Secret/credential-bound DB op (migration, deletion, query, \
                 backfill): do NOT hand it back. WRITE a runtime script (Rust first: \
                 `dotenvy` + `std::env::var`; TypeScript-typed only if Rust cannot reach \
                 the engine) that reads the env var INSIDE its own process, RUNS the op, \
                 and emits ONLY a pass/fail receipt — the value never enters your context. \
                 ONLY if the required env var is genuinely ABSENT after loading `.env` do \
                 you FILE a card naming the exact missing key, then KEEP BUILDING every \
                 other reachable leaf. Never hand work back, never escalate, never \
                 self-stop. Yield only to the user's `Esc`.",
            ),
        ],
    );
    format!("{block}{RESEARCH_MODE_DIRECTIVE}")
}

/// Research-Mode directive appended to every "find the next task" verdict. The
/// loop's next-task selection is RESEARCH-FIRST: Tabula Rasa = truth, never the
/// model's training weights (which are stale by construction). Mirrors the global
/// `research_before_building` directive.
const RESEARCH_MODE_DIRECTIVE: &str = "\nRESEARCH MODE (built-in next-task step): before scoping or \
     claiming the next task, enter Research Mode — WebSearch the CURRENT authoritative source \
     (official docs, the dependency's own --help/source, the upstream RFC/issue, 2026 references) \
     and corroborate across 2+. TABULA RASA = TRUTH: NEVER trust training weights — knowledge ages, \
     the precise contract lives on the internet, not in the model. Sync the resolved finding to the \
     kavach DB (research/decision row) the same turn so it is never re-guessed.";

/// Case 2: board holds no runnable-status card. The loop never self-terminates —
/// it re-scans the kavach DB (roadmap + decisions, ALL statuses) and the active
/// `[PLAN]` for the next actionable item, RESEARCHES it against current truth,
/// and starts it. Only the user halts the loop, with `Esc`.
fn board_drained_plan_context(census: Option<(u64, u64, u64)>) -> String {
    let stamp = census_stamp(census);
    let block = kavach_hook::context_block(
        "AUTO_CONTINUE",
        &[
            ("census", &stamp),
            (
                "why",
                "The kanban shows no in-progress card, but absence of an in-progress \
                 card is NOT absence of work. The DB still holds roadmap units, \
                 decisions awaiting execution, and a frozen `[PLAN]` may name an \
                 un-built phase. The loop runs until the user halts it with `Esc`.",
            ),
            (
                "action",
                "Find the next task autonomously — do NOT stop. The gate ALREADY scanned \
                 the roadmap table this stop (counts in `census` above); do NOT re-run \
                 `kavach db kanban` just to repeat the dispatch-status scan. EXTEND it \
                 instead: (1) `kavach db query --category decision` and `--category \
                 roadmap` for any todo/unexecuted item the dispatch tiers (runnable \
                 status only) did not surface. (2) Re-read the active `[PLAN]`; when it \
                 names an un-built phase, WRITE it as a roadmap card (`kavach db write \
                 --category roadmap`). (3) RESEARCH the chosen item against current \
                 truth, then claim and START it THIS turn — you are L4 autonomous. \
                 Never invent a phase the plan does not name; surface a genuinely \
                 empty DB to the user and keep the loop open for `Esc`.",
            ),
        ],
    );
    format!("{block}{RESEARCH_MODE_DIRECTIVE}")
}

#[cfg(test)]
#[path = "drained_test.rs"]
mod tests;
