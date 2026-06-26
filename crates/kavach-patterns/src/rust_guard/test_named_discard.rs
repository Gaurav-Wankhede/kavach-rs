//! Named-underscore discard tests (`RUST_P` index 78) — the `let _name = <expr>`
//! discarded-signal class. Exercised through the `detect()` engine entry point.
//! The DB/HTTP/await sub-cases (indices 50-52) are covered in `test_async_db.rs`;
//! these prove the GENERAL arm fires on any named discard and respects the RAII
//! exclusion the regex cannot express (no lookaround in this engine).
use crate::rust_guard::detect;

const PAT: &str = "let _name discards a return value";

#[test]
fn flags_named_discard_of_a_call() {
    // The exact dispatch work-steal bug shape: a lost-race bool thrown away.
    let v = detect(
        "src/lib.rs",
        "fn f() {\n    let _claimed = claim_card(p, k);\n}\n",
    );
    assert!(
        v.iter().any(|x| x.pattern == PAT),
        "named discard must fire"
    );
}

#[test]
fn flags_named_result_discard() {
    let v = detect(
        "src/lib.rs",
        "fn f() {\n    let _result = run_thing();\n}\n",
    );
    assert!(v.iter().any(|x| x.pattern == PAT));
}

const ANON: &str = "anonymous let _ = discards a call or live binding";

#[test]
fn flags_anonymous_call_discard() {
    // `let _ = fallible()` swallows the Result — the let_underscore_must_use class.
    let v = detect("src/lib.rs", "fn f() {\n    let _ = run_thing();\n}\n");
    assert!(
        v.iter().any(|x| x.pattern == ANON),
        "anon call discard must fire"
    );
}

#[test]
fn flags_anonymous_tuple_param_discard() {
    // The exact launder from the transcript: `let _ = (lat, lng);` suppresses live params.
    let v = detect(
        "src/lib.rs",
        "fn f(lat: f64, lng: f64) {\n    let _ = (lat, lng);\n}\n",
    );
    assert!(
        v.iter().any(|x| x.pattern == ANON),
        "tuple-of-bindings discard must fire"
    );
}

#[test]
fn allows_type_only_anonymous() {
    let v = detect("src/lib.rs", "fn f() {\n    let _: () = noop();\n}\n");
    assert!(!v.iter().any(|x| x.pattern == PAT));
}

#[test]
fn allows_anonymous_unit_discard() {
    // `let _ = ();` is a genuine no-op — not a call, not a binding-tuple: stays silent.
    let v = detect("src/lib.rs", "fn f() {\n    let _ = ();\n}\n");
    assert!(
        !v.iter().any(|x| x.pattern == ANON),
        "unit discard is benign"
    );
}

#[test]
fn allows_raii_guard_bindings() {
    // RAII held-for-drop names are filtered per-match in discard_race.rs. This
    // path now resolves the allow-list through `kavach_types::gate_patterns`
    // (`unit.gate-cfg-patterns-safelist-wireup`); with no daemon in-test the
    // resolver returns the compiled floor unchanged — so the floor must still
    // exempt every guard, proving the wireup is behavior-preserving fail-closed.
    let code = "fn f() {\n    let _guard = lock.lock();\n    let _span = enter();\n    let _permit = sem.acquire();\n}\n";
    let v = detect("src/lib.rs", code);
    assert!(
        !v.iter().any(|x| x.pattern == PAT),
        "RAII guards are held-for-drop, not discards — floor honored via gate_patterns"
    );
}
