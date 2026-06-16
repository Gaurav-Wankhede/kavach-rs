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
    }
}

fn parked_card(key: &str, marker: &str) -> MemoryEntry {
    let mut c = card(key, "todo", None);
    c.content = format!("{marker} owner-only, no agent code.");
    c
}

#[test]
fn parked_agent_blocked_card_is_not_selected() {
    let cards = vec![parked_card("p", "AGENT_BLOCKED:")];
    let picked = pick_in_lane(&cards, &cards, |e| lane_matches(e, None));
    assert!(picked.is_none(), "an AGENT_BLOCKED card must not dispatch");
}

#[test]
fn parked_owner_gated_card_is_not_selected() {
    let cards = vec![parked_card("p", "OWNER-GATED:")];
    let picked = pick_in_lane(&cards, &cards, |e| lane_matches(e, None));
    assert!(picked.is_none(), "an OWNER-GATED card must not dispatch");
}

#[test]
fn unparked_sibling_selected_over_parked_card() {
    let cards = vec![parked_card("parked", "OWNER-GATED:"), card("live", "todo", None)];
    let picked = pick_in_lane(&cards, &cards, |e| lane_matches(e, None));
    assert_eq!(picked.expect("the live card").key, "live");
}

fn umbrella_card(key: &str) -> MemoryEntry {
    let mut c = card(key, "todo", None);
    c.title = format!("{key} [UMBRELLA/EPIC — status child-derived]");
    c
}

#[test]
fn umbrella_parent_card_is_not_selected() {
    let cards = vec![umbrella_card("parent")];
    let picked = pick_in_lane(&cards, &cards, |e| lane_matches(e, None));
    assert!(picked.is_none(), "an UMBRELLA/EPIC parent must not dispatch");
}

#[test]
fn leaf_child_selected_over_umbrella_parent() {
    let cards = vec![umbrella_card("parent"), card("leaf", "todo", None)];
    let picked = pick_in_lane(&cards, &cards, |e| lane_matches(e, None));
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
    let p1 = pick_in_lane(&cards, &cards, |e| lane_matches(e, want));
    assert_eq!(p1.expect("own-lane card runnable").key, "mine");

    // Drain the own lane: pass 1 empty, pass 2 yields the unlaned card.
    let drained: Vec<MemoryEntry> = vec![
        card("mine", "done", Some("crypto")),
        card("theirs", "todo", Some("comms")),
        card("shared", "todo", None),
    ];
    let p1b = pick_in_lane(&drained, &drained, |e| lane_matches(e, want));
    assert!(p1b.is_none(), "own lane drained");
    let p2 = pick_in_lane(&drained, &drained, |e| e.lane.is_none());
    assert_eq!(p2.expect("unlaned fallback").key, "shared");

    // The foreign lane ("comms") is never selected by either pass.
    let foreign = pick_in_lane(&drained, &drained, |e| lane_matches(e, want))
        .or_else(|| pick_in_lane(&drained, &drained, |e| e.lane.is_none()));
    assert_ne!(foreign.expect("a card").key, "theirs");
}
