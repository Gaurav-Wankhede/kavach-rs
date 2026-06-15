use kavach_types::MemoryStatus;

use super::layout;
use crate::state::EntryRef;

fn card(key: &str, content: &str) -> EntryRef {
    EntryRef {
        project_slug: "kavach-rs".into(),
        category: "roadmap".into(),
        key: key.into(),
        title: key.into(),
        content: content.into(),
        status: MemoryStatus::Todo,
    }
}

#[test]
fn depless_cards_are_all_tier_zero() {
    let rows = vec![card("a", ""), card("b", "no deps here")];
    let (tiers, cyclic) = layout(&rows);
    assert!(cyclic.is_empty());
    assert_eq!(tiers.len(), 1, "no edges -> a single tier");
    assert_eq!(tiers[0].len(), 2);
}

#[test]
fn linear_chain_assigns_increasing_tiers() {
    // c depends on b depends on a -> tiers 0,1,2.
    let rows = vec![
        card("a", ""),
        card("b", "DEPENDS_ON: a"),
        card("c", "DEPENDS_ON: b"),
    ];
    let (tiers, cyclic) = layout(&rows);
    assert!(cyclic.is_empty());
    assert_eq!(tiers.len(), 3);
    assert_eq!(tiers[0][0].entry.key, "a");
    assert_eq!(tiers[1][0].entry.key, "b");
    assert_eq!(tiers[2][0].entry.key, "c");
    assert_eq!(tiers[2][0].deps, vec!["b".to_owned()]);
}

#[test]
fn fan_in_takes_deepest_prereq_tier() {
    // d depends on both a (tier 0) and c (tier 2) -> d is tier 3, not tier 1.
    let rows = vec![
        card("a", ""),
        card("b", "DEPENDS_ON: a"),
        card("c", "DEPENDS_ON: b"),
        card("d", "DEPENDS_ON: a, c"),
    ];
    let (tiers, _) = layout(&rows);
    assert_eq!(tiers.len(), 4);
    assert_eq!(tiers[3][0].entry.key, "d");
}

#[test]
fn cycle_is_surfaced_not_dropped_or_looped() {
    // a <-> b mutually depend -> neither resolves; both land in `cyclic`.
    let rows = vec![card("a", "DEPENDS_ON: b"), card("b", "DEPENDS_ON: a")];
    let (tiers, cyclic) = layout(&rows);
    assert_eq!(cyclic.len(), 2, "both cyclic cards surfaced, none dropped");
    assert!(tiers.iter().all(Vec::is_empty), "no cyclic card placed on a tier");
}

#[test]
fn absent_cross_project_dep_does_not_deepen_tier() {
    // x depends on a key NOT on the board -> not counted toward depth, so x is
    // tier 0 here.
    let rows = vec![card("x", "DEPENDS_ON: not-on-this-board")];
    let (tiers, cyclic) = layout(&rows);
    assert!(cyclic.is_empty());
    assert_eq!(tiers.len(), 1);
    assert_eq!(tiers[0][0].entry.key, "x");
    assert!(tiers[0][0].deps.is_empty(), "absent dep is not an on-board edge");
}

#[test]
fn empty_board_yields_one_empty_tier_no_panic() {
    let (tiers, cyclic) = layout(&[]);
    assert!(cyclic.is_empty());
    assert_eq!(tiers.len(), 1);
    assert!(tiers[0].is_empty());
}
