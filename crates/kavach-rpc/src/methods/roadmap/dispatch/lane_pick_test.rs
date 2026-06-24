use super::{lane_matches, pick_in_lane};
use kavach_surreal::MemoryEntry;

fn card(key: &str, status: &str, lane: Option<&str>) -> MemoryEntry {
    MemoryEntry {
        id: None,
        project: surrealdb_types::RecordId::new("project", "t"),
        category: Some("roadmap".into()),
        entry_key: key.into(),
        title: key.into(),
        content: String::new(),
        status: None,
        entry_status: Some(status.into()),
        tags: None,
        decay_score: None,
        access_count: None,
        created_at: None,
        updated_at: None,
        priority: None,
        lane: lane.map(Into::into),
        occupied_by: None,
        occupied_until: None,
    }
}

/// A `todo` card LIVE-leased by `holder` until `secs_from_now` (negative = expired).
fn leased_card(key: &str, holder: &str, secs_from_now: i64) -> MemoryEntry {
    let mut c = card(key, "todo", None);
    c.occupied_by = Some(holder.into());
    c.occupied_until =
        chrono::Utc::now().checked_add_signed(chrono::Duration::seconds(secs_from_now));
    c
}

fn marked_card(key: &str, marker: &str) -> MemoryEntry {
    let mut c = card(key, "todo", None);
    c.content = format!("{marker} operator-only, no agent code.");
    c
}

// PARKING ABOLISHED (operator directive 2026-06-16, reaffirmed 2026-06-17): the
// former `AGENT_BLOCKED:`/`OPERATOR-GATED:` content markers are INERT — they no
// longer suppress dispatch (the `is_parked` selector was removed). A card is
// runnable or DELETED. These tests pin the new contract: a runnable card is
// selected on status + deps + umbrella alone, regardless of any leftover marker
// text. They are the regression tripwire if a future change re-adds `is_parked`.

#[test]
fn agent_blocked_marker_does_not_suppress_dispatch() {
    let cards = vec![marked_card("p", "AGENT_BLOCKED:")];
    let picked = pick_in_lane(&cards, &cards, "", |e| lane_matches(e, None));
    assert_eq!(
        picked
            .expect("a runnable card dispatches despite the inert marker")
            .key,
        "p"
    );
}

#[test]
fn operator_gated_marker_does_not_suppress_dispatch() {
    let cards = vec![marked_card("p", "OPERATOR-GATED:")];
    let picked = pick_in_lane(&cards, &cards, "", |e| lane_matches(e, None));
    assert_eq!(
        picked
            .expect("a runnable card dispatches despite the inert marker")
            .key,
        "p"
    );
}

#[test]
fn priority_order_decides_between_two_runnable_cards() {
    // `entries` is pre-sorted by priority; the first match wins. A marker on the
    // first card no longer demotes it — both are runnable, the first is picked.
    let cards = vec![
        marked_card("first", "OPERATOR-GATED:"),
        card("second", "todo", None),
    ];
    let picked = pick_in_lane(&cards, &cards, "", |e| lane_matches(e, None));
    assert_eq!(picked.expect("the first runnable card").key, "first");
}

fn umbrella_card(key: &str) -> MemoryEntry {
    let mut c = card(key, "todo", None);
    c.title = format!("{key} [UMBRELLA/EPIC — status child-derived]");
    c
}

#[test]
fn umbrella_parent_card_is_not_selected() {
    let cards = vec![umbrella_card("parent")];
    let picked = pick_in_lane(&cards, &cards, "", |e| lane_matches(e, None));
    assert!(
        picked.is_none(),
        "an UMBRELLA/EPIC parent must not dispatch"
    );
}

#[test]
fn leaf_child_selected_over_umbrella_parent() {
    let cards = vec![umbrella_card("parent"), card("leaf", "todo", None)];
    let picked = pick_in_lane(&cards, &cards, "", |e| lane_matches(e, None));
    assert_eq!(picked.expect("the leaf child").key, "leaf");
}

#[test]
fn lane_matches_unset_session_matches_every_card() {
    assert!(lane_matches(&card("a", "todo", Some("crypto")), None));
    assert!(lane_matches(&card("b", "todo", None), None));
}

#[test]
fn lane_matches_only_same_lane() {
    let c = card("a", "todo", Some("crypto"));
    assert!(lane_matches(&c, Some("crypto")));
    assert!(!lane_matches(&c, Some("comms")));
    // an unlaned card does NOT match a named session lane (it's the pass-2 pool)
    assert!(!lane_matches(&card("b", "todo", None), Some("crypto")));
}

#[test]
fn two_pass_picks_own_lane_then_unlaned_never_foreign() {
    let cards = vec![
        card("mine", "todo", Some("crypto")),
        card("theirs", "todo", Some("comms")),
        card("shared", "todo", None),
    ];
    let want = Some("crypto");

    // Pass 1 (own lane) wins over the unlaned fallback.
    let p1 = pick_in_lane(&cards, &cards, "", |e| lane_matches(e, want));
    assert_eq!(p1.expect("own-lane card runnable").key, "mine");

    // Drain the own lane: pass 1 empty, pass 2 yields the unlaned card.
    let drained: Vec<MemoryEntry> = vec![
        card("mine", "done", Some("crypto")),
        card("theirs", "todo", Some("comms")),
        card("shared", "todo", None),
    ];
    let p1b = pick_in_lane(&drained, &drained, "", |e| lane_matches(e, want));
    assert!(p1b.is_none(), "own lane drained");
    let p2 = pick_in_lane(&drained, &drained, "", |e| e.lane.is_none());
    assert_eq!(p2.expect("unlaned fallback").key, "shared");

    // The foreign lane ("comms") is never selected by either pass.
    let foreign = pick_in_lane(&drained, &drained, "", |e| lane_matches(e, want))
        .or_else(|| pick_in_lane(&drained, &drained, "", |e| e.lane.is_none()));
    assert_ne!(foreign.expect("a card").key, "theirs");
}

// ── Multi-session task-steal guard (operator directive 2026-06-18) ──────────────
// Two terminals (project + research) MUST NOT grab each other's live card. A
// card LIVE-leased by a DIFFERENT session is skipped; my own / expired / unleased
// cards remain selectable.

#[test]
fn live_lease_by_other_session_is_skipped() {
    // Terminal B (sess-B) selecting: a card sess-A holds live must NOT be picked.
    let cards = vec![leased_card("a-card", "sess-A", 300)];
    let picked = pick_in_lane(&cards, &cards, "sess-B", |e| lane_matches(e, None));
    assert!(
        picked.is_none(),
        "a card live-leased by another session must not dispatch"
    );
}

#[test]
fn my_own_live_lease_is_still_selectable() {
    // Re-dispatch of MY own in-progress card is fine (resume, not steal).
    let cards = vec![leased_card("mine", "sess-A", 300)];
    let picked = pick_in_lane(&cards, &cards, "sess-A", |e| lane_matches(e, None));
    assert_eq!(picked.expect("my own card resumes").key, "mine");
}

#[test]
fn expired_lease_is_selectable_by_anyone() {
    // An expired lease (crashed/abandoned holder) is free — the reclaim path.
    let cards = vec![leased_card("orphan", "sess-A", -10)];
    let picked = pick_in_lane(&cards, &cards, "sess-B", |e| lane_matches(e, None));
    assert_eq!(picked.expect("expired lease is free").key, "orphan");
}

#[test]
fn other_session_card_skipped_unlaned_card_picked() {
    // The exact two-terminal bug: B skips A's live card, takes the free one.
    let cards = vec![
        leased_card("a-active", "sess-A", 300),
        card("free", "todo", None),
    ];
    let picked = pick_in_lane(&cards, &cards, "sess-B", |e| lane_matches(e, None));
    assert_eq!(picked.expect("B takes the un-leased card").key, "free");
}

#[test]
fn empty_me_treats_any_live_lease_as_foreign_fail_closed() {
    // An un-identified session (no KAVACH_SESSION_ID) must NOT steal a live card.
    let cards = vec![leased_card("held", "sess-A", 300)];
    let picked = pick_in_lane(&cards, &cards, "", |e| lane_matches(e, None));
    assert!(
        picked.is_none(),
        "empty session id fails closed — never steals a live card"
    );
}

// ── Stale-claim sweep predicate (E4, operator directive 2026-06-18) ─────────────
// A card stuck `in_progress` with an EXPIRED lease = a crashed session's orphan;
// the dispatch sweep resets it to `todo`. A live lease or an un-leased card is NOT
// stale.

/// An `in_progress` card whose lease lapsed `abs(secs)` ago (secs negative = expired).
fn claimed_card(key: &str, holder: &str, secs_from_now: i64) -> MemoryEntry {
    let mut c = card(key, "in_progress", None);
    c.occupied_by = Some(holder.into());
    c.occupied_until =
        chrono::Utc::now().checked_add_signed(chrono::Duration::seconds(secs_from_now));
    c
}

#[test]
fn expired_lease_on_in_progress_is_a_stale_claim() {
    assert!(claimed_card("orphan", "dead-sess", -10).is_stale_claim());
}

#[test]
fn live_lease_on_in_progress_is_not_stale() {
    assert!(
        !claimed_card("active", "live-sess", 300).is_stale_claim(),
        "holder still working"
    );
}

#[test]
fn unleased_in_progress_is_not_swept() {
    let mut c = card("manual", "in_progress", None);
    c.occupied_until = None; // no lease to prove abandonment
    assert!(!c.is_stale_claim());
}

#[test]
fn a_todo_card_is_never_a_stale_claim() {
    assert!(
        !leased_card("t", "s", -10).is_stale_claim(),
        "todo is not in_progress"
    );
}
