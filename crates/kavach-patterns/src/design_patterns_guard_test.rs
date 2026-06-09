use super::*;

#[test]
fn detects_deny_warnings() {
    let code = "#![deny(warnings)]\nfn main() {}";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("deny(warnings)")));
}

#[test]
fn detects_string_param() {
    let code = "fn parse(s: &String) -> usize { s.len() }";
    let v = detect("src/util.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("&String")));
}

#[test]
fn detects_vec_param() {
    let code = "fn sum(v: &Vec<i32>) -> i32 { v.iter().sum() }";
    let v = detect("src/util.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("&Vec")));
}

#[test]
fn detects_many_arg_new() {
    let code =
        "impl Foo { pub fn new(a: i32, b: i32, c: i32, d: i32, e: i32) -> Self { Self {} } }";
    let v = detect("src/foo.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("Builder")));
}

#[test]
fn allows_str_param() {
    let code = "fn parse(s: &str) -> usize { s.len() }";
    let v = detect("src/util.rs", code);
    assert!(!v.iter().any(|x| x.pattern.contains("&String")));
}

#[test]
fn skips_test_files() {
    let code = "#![deny(warnings)]";
    let v = detect("src/tests/mod.rs", code);
    assert!(v.is_empty());
}

#[test]
fn detects_string_param_multiarg() {
    // Edge case: &String in second position should still match
    let code = "fn parse(s: &str, t: &String) -> usize { s.len() + t.len() }";
    let v = detect("src/util.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("&String")));
}

#[test]
fn rejects_deref_on_non_smartpointer() {
    // impl Deref is matched regardless of whether it's a smart pointer
    // This is acceptable as P2 (advisory) — false positives are okay for heuristics
    let code = "impl Deref for MyNotAPtr { type Target = Inner; }";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("impl Deref")));
}

#[test]
fn rejects_four_comma_constructor() {
    // 4 commas = 5 args; should trigger Builder advisory
    let code = "fn new(a: i32, b: i32, c: i32, d: i32, e: i32) -> Self { Self {} }";
    let v = detect("src/foo.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("Builder")));
}

#[test]
fn allows_three_arg_constructor() {
    // 3 args (2 commas); should NOT trigger
    let code = "fn new(a: i32, b: i32, c: i32) -> Self { Self {} }";
    let v = detect("src/foo.rs", code);
    assert!(!v.iter().any(|x| x.pattern.contains("Builder")));
}

// ---- GoF pattern advisories (refactoring.guru/design-patterns/rust) ----

#[test]
fn detects_singleton_static_mut() {
    // GoF Singleton: `static mut` is unsound — steer to OnceLock/LazyLock.
    let code = "static mut COUNTER: u32 = 0;";
    let v = detect("src/g.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("Singleton")));
}

#[test]
fn detects_state_match_on_field() {
    // GoF State/Strategy: `match self.state { … }` is the behaviour-by-field smell.
    let code = "fn step(&self) { match self.state { A => 1, B => 2 } }";
    let v = detect("src/g.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("State")));
}

#[test]
fn detects_manual_index_walk() {
    // GoF Iterator: manual `i += 1` index walk reimplements Iterator.
    let code = "fn f() { let mut i = 0; while i < n { i += 1; } }";
    let v = detect("src/g.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("Iterator")));
}

#[test]
fn detects_observer_callback_list() {
    // GoF Observer: hand-rolled `Vec<Box<dyn Fn…>>` callback registry.
    let code = "struct S { subs: Vec<Box<dyn Fn(u32)>> }";
    let v = detect("src/g.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("Observer")));
}

#[test]
fn detects_state_take_on_boxdyn() {
    // GoF State: `mem::take` on `Box<dyn _>` field fails E0277 (no Default).
    let code = "self.state = std::mem::take(&mut self.state).play();";
    let v = detect("src/g.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("State transition")));
}

#[test]
fn detects_flyweight_mut_returns_ref() {
    // GoF Flyweight: `fn get(&mut self, …) -> &Cache` aliases the &mut borrow (E0499).
    let code = "fn get_tree_type(&mut self, key: &str) -> &TreeType { todo!() }";
    let v = detect("src/g.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("Flyweight")));
}

#[test]
fn allows_clean_singleton_oncelock() {
    // OnceLock-based singleton must NOT trip the static-mut rule.
    let code = "static CFG: OnceLock<Config> = OnceLock::new();";
    let v = detect("src/g.rs", code);
    assert!(!v.iter().any(|x| x.pattern.contains("Singleton")));
}

#[test]
fn allows_clean_iterator_impl() {
    // A proper Iterator impl must NOT trip the manual-index heuristic.
    let code = "impl Iterator for S { type Item = u8; fn next(&mut self) -> Option<u8> { None } }";
    let v = detect("src/g.rs", code);
    assert!(!v.iter().any(|x| x.pattern.contains("Iterator")));
}
