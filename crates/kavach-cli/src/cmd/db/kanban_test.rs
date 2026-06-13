use super::*;

#[test]
fn count_non_open_empty_input_returns_zero_for_all() {
    let counts = count_non_open(std::iter::empty::<&str>());
    assert_eq!(counts, EmptyKanbanCounts::default());
}

#[test]
fn count_non_open_separates_terminal_and_unparseable_statuses() {
    // Anything outside the 4 canonical states {todo, in_progress, done,
    // verified} is unparseable — e.g. a stale string from a pre-collapse row.
    let counts = count_non_open([
        "verified", "verified", "verified", "legacy-a", "legacy-b", "garbage", "??",
    ]);
    assert_eq!(counts.verified, 3);
    assert_eq!(
        counts.unparseable, 4,
        "every non-canonical status counts as unparseable"
    );
}

#[test]
fn count_non_open_ignores_open_statuses_and_counts_unparseable() {
    let counts = count_non_open(["todo", "in_progress", "done", "garbage", "stale-status"]);
    assert_eq!(
        counts.unparseable, 2,
        "two non-canonical strings are both unparseable"
    );
    assert_eq!(counts.verified, 0);
}

#[test]
fn count_non_open_surfaces_corruption_signal() {
    let counts = count_non_open(["", "NULL", "Done", "DONE", "DEFERRED"]);
    assert_eq!(counts.unparseable, 5);
}

#[test]
fn hunt_key_partition_predicate() {
    assert!(is_hunt_key("hunt.rpc-socket-no-auth"));
    assert!(is_hunt_key("hunt.x"));
    assert!(!is_hunt_key("P8-ws-backlog"));
    assert!(!is_hunt_key("hunting-without-dot"));
    assert!(!is_hunt_key(""));
    // Stdlib partition splits the two lenses by this exact predicate.
    let keys = ["hunt.a", "roadmap-1", "hunt.b", "P2-feature"];
    let (hunt, roadmap): (Vec<&str>, Vec<&str>) =
        keys.iter().copied().partition(|k| is_hunt_key(k));
    assert_eq!(hunt, ["hunt.a", "hunt.b"]);
    assert_eq!(roadmap, ["roadmap-1", "P2-feature"]);
}

#[test]
fn done_prefix_filter_logic() {
    assert!(is_done_title("DONE: task"));
    assert!(is_done_title("DONE task"));
    assert!(is_done_title("DONE-task"));
    assert!(is_done_title("DONE_task"));
    assert!(!is_done_title("DONEtask"));
    assert!(!is_done_title("done: task"));
    assert!(!is_done_title("open task"));
    assert!(!is_done_title(""));
}
