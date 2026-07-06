use super::*;

#[test]
fn detects_hex() {
    let code = ["bg-[#3b82", "f6]"].concat();
    assert!(check("src/C.tsx", &code).is_some());
}

#[test]
fn allows_semantic() {
    assert!(check("src/C.tsx", "className=\"bg-blue-500\"").is_none());
}

#[test]
fn skips_rust() {
    assert!(check("src/main.rs", "bg-[#fff]").is_none());
}
