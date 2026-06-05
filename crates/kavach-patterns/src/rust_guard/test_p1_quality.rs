//! P1 quality-nudge detector tests (encapsulation/capacity/generics/bool-param).
use crate::rust_guard::detect;

#[test]
fn p1_pub_fields_encapsulation() {
    let code = "pub name: String,\npub age: i32,\npub email: String,\npub active: bool,";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("encapsulation")));
}

#[test]
fn p1_vec_no_capacity() {
    let code = "let mut items = Vec::new();\nfor x in data {\n    items.push(x);\n}";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern.contains("capacity")));
}

#[test]
fn p1_concrete_type_param() {
    let v = detect("src/lib.rs", "fn process(data: &Vec<String>) {}");
    assert!(v.iter().any(|x| x.pattern.contains("concrete type")));
}

#[test]
fn p1_manual_type_dispatch() {
    let v = detect("src/lib.rs", "if val.downcast::<Foo>().is_ok() {}");
    assert!(v.iter().any(|x| x.pattern.contains("type dispatch")));
}

#[test]
fn p1_bool_param() {
    let v = detect("src/lib.rs", "fn send_email(user: &User, urgent: bool) {}");
    assert!(v.iter().any(|x| x.pattern == "bool parameter"));
}

#[test]
fn p1_unused_param_suppression() {
    let code = "fn send_email(_email_client: &Option<IronGateEmailClient>, to: &str) { }";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern == "unused param suppression"));
}

#[test]
fn ok_when_param_used() {
    let code = "fn send_email(email_client: &Option<IronGateEmailClient>, to: &str) { }";
    let v = detect("src/lib.rs", code);
    assert!(!v.iter().any(|x| x.pattern == "unused param suppression"));
}
