use super::*;

#[test]
fn clean_code_passes() {
    assert!(check_for_hallucinations("use std::io;").is_none());
}

#[test]
fn fake_url_detected() {
    let r = check_for_hallucinations("let url = \"https://api.example.com/v1\";");
    assert!(r.is_some());
    assert!(r.unwrap().contains("HALLUCINATION_WARNING"));
}

#[test]
fn placeholder_key_detected() {
    let r = check_for_hallucinations("let key = \"sk-your-api-key\";");
    assert!(r.is_some());
}

#[test]
fn multiple_issues_reported() {
    let code = "let url = \"https://your-api.com\";\nlet key = YOUR_API_KEY;";
    let msg = check_for_hallucinations(code).unwrap();
    assert!(msg.contains("Fake URL"));
    assert!(msg.contains("Placeholder"));
}
