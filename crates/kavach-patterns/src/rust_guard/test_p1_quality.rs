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

#[test]
fn p1_string_parameter_by_value() {
    let v = detect("src/lib.rs", "fn set_name(name: String) {}");
    assert!(v.iter().any(|x| x.pattern == "String parameter by-value"));
}

#[test]
fn p1_c_style_index_loop() {
    let v = detect("src/lib.rs", "for i in 0..len { println!(\"{}\", i); }");
    assert!(v.iter().any(|x| x.pattern == "C-style index loop"));
}

#[test]
fn p1_arc_mutex_overuse() {
    let v = detect("src/lib.rs", "let state = Arc::new(Mutex::new(0));");
    assert!(v.iter().any(|x| x.pattern == "Arc<Mutex<T>> overuse"));
}

#[test]
fn p1_primitive_obsession() {
    let v = detect("src/lib.rs", "fn process(user_id: u32, account_id: u32) {}");
    assert!(v.iter().any(|x| x.pattern == "primitive obsession"));
}

#[test]
fn p1_string_concat_with_plus() {
    let v = detect("src/lib.rs", "let s = greeting + &name;");
    assert!(v.iter().any(|x| x.pattern == "String concatenation with +"));
}

#[test]
fn p1_fighting_borrow_checker_nested_lock() {
    let v = detect("src/lib.rs", "let cache: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));");
    assert!(v.iter().any(|x| x.pattern == "fighting the borrow checker (nested shared lock)"));
}
