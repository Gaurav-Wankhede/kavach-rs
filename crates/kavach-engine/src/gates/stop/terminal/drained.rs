//! Shared drained-board terminal verdict — the SINGLE source of truth both stop
//! terminals emit when the dispatch tiers find no runnable card.
//!
//! Three states hide behind "nothing dispatchable" with DIFFERENT outcomes. None
//! tells the LLM to stop: the loop runs until the user halts it with `Esc`.
//!
//! 0. The session is pinned to a lane (`KAVACH_LANE`) and its lane + the unlaned
//!    backlog are both drained → `[LANE_DRAINED]` (lane.rs). Never cross into a
//!    foreign lane; that is another session's work.
//! 1. No runnable-status card. Re-scan roadmap + decisions (ALL statuses) and the
//!    active `[PLAN]` for the next actionable item → `[AUTO_CONTINUE]`.
//!
//! A board whose every runnable card is dependency-blocked is NOT a terminal state
//! and has NO status tag: it is a blocker to WALK and BUILD, handled by a
//! refuse-stop in `clean_exit` (`board_is_all_blocked`), identical in spirit to the
//! cycle-deadlock refuse. There is no "everything is blocked, so the turn may end".
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
    // A dependency cycle is a deadlock the AI must repair (break the cycle), never a
    // clean stop. An all-blocked board is the SAME shape — walk to the blocker and
    // build it — and is refused upstream in `clean_exit` (`board_is_all_blocked`).
    // Both surface before the PLAN nudge so neither can forge a clean stop.
    if census.is_some_and(|(_, _, cyclic)| cyclic > 0) || census_is_all_blocked(census) {
        return blocker_walk_context();
    }
    board_drained_plan_context(census)
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

/// Public all-blocked check for `clean_exit`: every runnable card blocked by deps
/// (or a cycle). When true, `clean_exit` REFUSES the stop with `blocker_walk_context`
/// — an all-blocked board is a blocker to BUILD, never a terminal.
#[must_use]
pub(in crate::gates::stop) fn board_is_all_blocked(project: &str) -> bool {
    let census = crate::gates::stop_dispatch::open_set_census(project);
    census.is_some_and(|(_, _, cyclic)| cyclic > 0) || census_is_all_blocked(census)
}

/// The single refuse-stop directive for a fully-blocked board (every runnable card
/// held by a dependency or a cycle). NOT a stop, NOT a hand-off: WALK to the
/// blocker and BUILD it leaf-first; a stale edge gets corrected, a cycle broken, a
/// missing credential FILED while the loop keeps draining. Yields only to `Esc`.
pub(in crate::gates::stop) fn blocker_walk_context() -> String {
    kavach_hook::context_block(
        "BLOCKER_WALK",
        &[(
            "action",
            "Do NOT stop. This walk fires ONLY because auto/bypassPermissions is live \
                 (attended modes already took the clean stop upstream) — so the loop is \
                 yours to drive, not the user's to approve. Every runnable card is ORDERED \
                 behind a `DEPENDS_ON:` edge \
                 or a cycle — that is work to RESOLVE, never a parked 'blocked' state and \
                 never a clean stop. There is NO blocked-and-waiting card: each one is \
                 either built now or its edge is resolved. WALK it: (1) read each card's \
                 `DEPENDS_ON:` line, get the prerequisite (`kavach db get --category \
                 roadmap --key <dep>`). (2) BUILD the prerequisite THIS turn — recurse to \
                 ITS dep until a leaf dispatches, then dispatch the dependent. (3) The dep \
                 is already DONE: drop the satisfied edge and dispatch. (4) The dep is \
                 STALE/obsolete (superseded, never coming): UPDATE the card to the current \
                 version, or REMOVE it from the todos (`kavach db status-update \
                 --status verified` / `kavach db delete`) — never leave it blocked. \
                 (5) CYCLE (A->B->A): `kavach db kanban --format mermaid`, then edit the \
                 offending `DEPENDS_ON:` to cut the back-edge. (6) Secret/credential-bound \
                 op: WRITE a runtime script (Rust + `dotenvy`) that reads the env var in \
                 its own process and emits ONLY a pass/fail receipt. ONLY a genuinely \
                 ABSENT env var is FILED as a card; then KEEP BUILDING every reachable \
                 leaf. Yield only to `Esc`.",
        )],
    )
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
#[cfg(test)]
#[path = "drained_test.rs"]
mod tests;