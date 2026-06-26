// Version-scheme-aware stability gate proofs: 0.x minor=breaking, 1.x+ major=breaking.
use super::{BumpKind, Version, classify_bump};

fn v(s: &str) -> Version {
    Version::parse(s).expect("parses")
}

#[test]
fn parse_tolerates_operators_and_suffixes() {
    assert_eq!(v("^1.2.3"), v("1.2.3"));
    assert_eq!(
        v("v0.7"),
        Version {
            major: 0,
            minor: 7,
            patch: 0
        }
    );
    assert_eq!(
        v("1"),
        Version {
            major: 1,
            minor: 0,
            patch: 0
        }
    );
    assert_eq!(v("0.7.9-rc.1"), v("0.7.9"));
    assert_eq!(v("2.0.0+build5"), v("2.0.0"));
}

#[test]
fn parse_rejects_non_numeric() {
    assert!(Version::parse("latest").is_none());
    assert!(Version::parse("").is_none());
}

#[test]
fn zero_ver_minor_bump_is_breaking() {
    // Dioxus 0.7.9 -> 0.8.0: minor moved in 0.x ⇒ BREAKING.
    assert_eq!(classify_bump(v("0.7.9"), v("0.8.0")), BumpKind::Breaking);
}

#[test]
fn zero_ver_patch_bump_is_compatible() {
    // 0.7.9 -> 0.7.10: only patch moved in 0.x ⇒ COMPATIBLE.
    assert_eq!(classify_bump(v("0.7.9"), v("0.7.10")), BumpKind::Compatible);
}

#[test]
fn one_plus_minor_bump_is_compatible() {
    // 1.2.0 -> 1.5.0: minor moved in 1.x ⇒ COMPATIBLE (standard SemVer).
    assert_eq!(classify_bump(v("1.2.0"), v("1.5.0")), BumpKind::Compatible);
}

#[test]
fn one_plus_major_bump_is_breaking() {
    assert_eq!(classify_bump(v("1.9.9"), v("2.0.0")), BumpKind::Breaking);
}

#[test]
fn stabilization_to_one_zero_is_breaking() {
    // 0.9.x -> 1.0.0: leaving the 0.x regime is the stabilization boundary.
    assert_eq!(classify_bump(v("0.9.5"), v("1.0.0")), BumpKind::Breaking);
}

#[test]
fn downgrade_and_noop_are_no_forward_change() {
    assert_eq!(
        classify_bump(v("1.5.0"), v("1.2.0")),
        BumpKind::NoForwardChange
    );
    assert_eq!(
        classify_bump(v("0.7.9"), v("0.7.9")),
        BumpKind::NoForwardChange
    );
}

#[test]
fn zero_major_bump_is_breaking() {
    // 0.7 -> 1-shaped-as-0? guard the rare 0.x major move (0.7 -> 0.x stays 0).
    // A 0.x -> 0.x major change cannot happen (major is always 0), but a wide
    // minor jump still trips breaking.
    assert_eq!(classify_bump(v("0.1.0"), v("0.99.0")), BumpKind::Breaking);
}
