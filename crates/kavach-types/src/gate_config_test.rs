use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn miss_falls_back_to_default() {
    let miss = |_: &str, _: &str| None;
    assert!((gate_threshold(miss, "p", "k", 0.85) - 0.85).abs() < f64::EPSILON);
    assert!(gate_enabled(miss, "p", "k", true));
    assert_eq!(gate_text(miss, "p", "k", "D"), "D");
}

#[test]
fn empty_inputs_short_circuit() {
    let called = AtomicBool::new(false);
    let boom = |_: &str, _: &str| -> Option<GateValueDto> {
        called.store(true, Ordering::SeqCst);
        None
    };
    assert!((gate_threshold(boom, "", "k", 1.0) - 1.0).abs() < f64::EPSILON);
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn hit_replaces_default() {
    let hit = |_: &str, _: &str| {
        Some(GateValueDto {
            kind: "threshold".to_owned(),
            num: Some(0.42),
            boolean: None,
            list: None,
            text: None,
        })
    };
    assert!((gate_threshold(hit, "p", "k", 0.85) - 0.42).abs() < f64::EPSILON);
}

#[test]
fn patterns_are_additive_floor_immutable() {
    let extra = |_: &str, _: &str| {
        Some(GateValueDto {
            kind: "pattern_list".to_owned(),
            num: None,
            boolean: None,
            list: Some(vec!["rm -rf".to_owned(), "custom".to_owned()]),
            text: None,
        })
    };
    let floor = ["rm -rf", "DROP TABLE"];
    let resolved = gate_patterns(extra, "p", "k", &floor);
    assert_eq!(
        resolved,
        vec![
            "rm -rf".to_owned(),
            "DROP TABLE".to_owned(),
            "custom".to_owned()
        ]
    );
}
