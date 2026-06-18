//! Tests for `detect_antiprod` and `detect_mock_data`.
use super::*;
use crate::config::j;

#[test]
fn test_p1() {
    let m = j(&["// ", "TO", "DO", ": later"]);
    assert!(
        detect_antiprod("a.ts", &m)
            .iter()
            .any(|x| x.level == AntiProdLevel::P1ProdLeak)
    );
}

#[test]
fn test_p2() {
    assert!(
        detect_antiprod("a.ts", "promise.catch(() => {})")
            .iter()
            .any(|x| x.level == AntiProdLevel::P2ErrorBlind)
    );
}

#[test]
fn test_p3() {
    assert!(
        detect_antiprod("a.ts", "const x = foo as any;")
            .iter()
            .any(|x| x.level == AntiProdLevel::P3TypeLoose)
    );
}

#[test]
fn test_allow() {
    let m = j(&["// ", "TO", "DO"]);
    assert!(detect_antiprod("src/tests/a.test.ts", &m).is_empty());
}

#[test]
fn test_docker() {
    let r = detect_antiprod("Dockerfile", "FROM node:latest\nRUN npm i");
    assert!(r.iter().any(|x| x.match_text.contains("latest")));
}

#[test]
fn test_mock() {
    assert!(detect_mock_data("a.tsx", "const mockUsers = [").is_some());
}

#[test]
fn test_501_rust() {
    assert!(
        detect_antiprod("h.rs", "StatusCode::NOT_IMPLEMENTED")
            .iter()
            .any(|x| x.code == "STUB_501")
    );
}

#[test]
fn test_501_ts() {
    assert!(
        detect_antiprod("h.ts", "res.status(501)")
            .iter()
            .any(|x| x.code == "STUB_501")
    );
}

#[test]
fn test_stub_body_in_handler() {
    assert!(
        detect_antiprod("src/handler.rs", r#""not implemented""#)
            .iter()
            .any(|x| x.code == "STUB_BODY")
    );
}

#[test]
fn empty_response_in_prod_handler_still_flags() {
    // EMPTY_RESPONSE (pattern 58) MUST still fire on a production handler.
    let code = "async fn get_user() -> impl IntoResponse {\n    Json(json!({}))\n}";
    assert!(
        detect_antiprod("src/routes/handler.rs", code)
            .iter()
            .any(|x| x.code == "EMPTY_RESPONSE"),
        "EMPTY_RESPONSE must still fire on a production handler"
    );
}

#[test]
fn empty_response_in_tests_dir_handler_is_not_flagged() {
    // Same content under tests/ is a fixture — NO EMPTY_RESPONSE false-positive.
    let code = "async fn get_user() -> impl IntoResponse {\n    Json(json!({}))\n}";
    assert!(
        !detect_antiprod("crates/x/tests/roundtrip_handler.rs", code)
            .iter()
            .any(|x| x.code == "EMPTY_RESPONSE"),
        "tests/ path must not raise EMPTY_RESPONSE"
    );
}

#[test]
fn empty_response_in_tools_binary_main_is_not_flagged() {
    // A CLI/migrator binary under /tools/ is NOT an HTTP handler — its `main`
    // ending in `Ok(())` is correct, not an "empty response". FP that blocked dbx.
    let code = "async fn main() -> anyhow::Result<()> {\n    run().await?;\n    Ok(())\n}";
    assert!(
        !detect_antiprod("crates/tools/dbx/src/main.rs", code)
            .iter()
            .any(|x| x.code == "EMPTY_RESPONSE"),
        "/tools/ binary main must not raise EMPTY_RESPONSE"
    );
}

#[test]
fn empty_response_under_cfg_test_is_not_flagged() {
    // A #[cfg(test)] module inside a handler file is a test context — no FP.
    let code = "#[cfg(test)]\nmod tests {\n    async fn get_user() -> impl IntoResponse { Json(json!({})) }\n}";
    assert!(
        !detect_antiprod("src/routes/handler.rs", code)
            .iter()
            .any(|x| x.code == "EMPTY_RESPONSE"),
        "#[cfg(test)] module must not raise EMPTY_RESPONSE"
    );
}

#[test]
fn test_n_plus_1_in_handler() {
    let code = "for user in users {\n    let posts = query(\"SELECT id, name FROM posts WHERE user_id = $1\");\n}";
    assert!(
        detect_antiprod("src/handler.rs", code)
            .iter()
            .any(|x| x.code == "N_PLUS_1")
    );
}

#[test]
fn test_nested_loop_in_handler() {
    let code =
        "for a in items {\n    for b in items {\n        if a == b { count += 1; }\n    }\n}";
    assert!(
        detect_antiprod("src/handler.rs", code)
            .iter()
            .any(|x| x.code == "NESTED_LOOP")
    );
}

#[test]
fn test_empty_response_in_handler() {
    let j = j;
    let code = j(&["Ok(Json(json!(", "{})))"]);
    assert!(
        detect_antiprod("src/handler.rs", &code)
            .iter()
            .any(|x| x.code == "EMPTY_RESPONSE")
    );
}

#[test]
fn test_status_misuse_in_handler() {
    assert!(
        detect_antiprod("src/handler.rs", "StatusCode::NOT_IMPLEMENTED")
            .iter()
            .any(|x| x.level == AntiProdLevel::P0MockData)
    );
}

#[test]
fn should_not_flag_empty_response_in_library_module() {
    let code = "pub fn sync() -> Result<()> { Ok(()) }";
    let hits = detect_antiprod("src/sync_logic.rs", code);
    assert!(!hits.iter().any(|x| x.code == "EMPTY_RESPONSE"));
}

#[test]
fn should_not_flag_nested_loop_in_tree_traversal_module() {
    let code =
        "for tree in &trees {\n    for child in &tree.children {\n        visit(child);\n    }\n}";
    let hits = detect_antiprod("src/walker.rs", code);
    assert!(!hits.iter().any(|x| x.code == "NESTED_LOOP"));
}

#[test]
fn should_not_flag_stub_body_in_doc_comment() {
    let code = r#"/// Matches "placeholder" keyword in the RAG metadata."#;
    let hits = detect_antiprod("src/matcher.rs", code);
    assert!(!hits.iter().any(|x| x.code == "STUB_BODY"));
}

#[test]
fn test_api_drift_frontend() {
    let content = "/** NOT_IMPLEMENTED: Backend route /soundbak/users/{id}/likes — returns empty array fallback */\ngetUserLikes: async () => adaptResponse(data, 'posts');";
    let hits = detect_antiprod("src/lib/api/soundbak.ts", content);
    assert!(
        hits.iter().any(|x| x.code == "API_DRIFT"),
        "should flag NOT_IMPLEMENTED in frontend API file"
    );
}

#[test]
fn test_api_drift_not_fired_in_rust() {
    let content = "// NOT_IMPLEMENTED: placeholder";
    let hits = detect_antiprod("src/handler.rs", content);
    assert!(
        !hits.iter().any(|x| x.code == "API_DRIFT"),
        "API_DRIFT must not fire on Rust files"
    );
}

#[test]
fn test_api_drift_not_fired_in_ui_component() {
    let content = "// NOT_IMPLEMENTED yet";
    let hits = detect_antiprod("src/components/Button.tsx", content);
    assert!(
        !hits.iter().any(|x| x.code == "API_DRIFT"),
        "API_DRIFT must not fire on UI components"
    );
}

#[test]
fn test_hardcoded_url_in_api_client() {
    let content = r#"fetch("https://api.example.com/users")"#;
    let hits = detect_antiprod("src/api/users.ts", content);
    assert!(
        hits.iter().any(|x| x.code == "HARDCODED_URL"),
        "should flag hardcoded https URL in API client"
    );
}

#[test]
fn test_hardcoded_url_not_fired_for_localhost() {
    let content = r#"fetch("http://localhost:3000/users")"#;
    let hits = detect_antiprod("src/api/users.ts", content);
    assert!(
        !hits.iter().any(|x| x.code == "HARDCODED_URL"),
        "localhost must not trigger HARDCODED_URL"
    );
}

#[test]
fn test_empty_fetch_in_api_client() {
    let content = "async function getUsers() {\n  return [];\n}";
    let hits = detect_antiprod("src/api/users.ts", content);
    assert!(
        hits.iter().any(|x| x.code == "EMPTY_FETCH"),
        "should flag hardcoded empty return in API client"
    );
}

#[test]
fn test_print_macro_rust() {
    let j = j;
    let p = j(&["pri", "nt!", "(\"leak\")"]);
    assert!(
        detect_antiprod("h.rs", &p)
            .iter()
            .any(|x| x.match_text.contains("print-macro"))
    );
}

#[test]
fn test_eprint_macro_rust() {
    let j = j;
    let e = j(&["epr", "int!", "(\"err\")"]);
    assert!(
        detect_antiprod("h.rs", &e)
            .iter()
            .any(|x| x.match_text.contains("print-macro"))
    );
}

#[test]
fn test_lowercase_prose_word_is_not_a_task_marker() {
    let j = j;
    let prose = format!("// resume this {} later when unblocked", j(&["to", "do"]));
    assert!(
        !detect_antiprod("src/x.rs", &prose)
            .iter()
            .any(|x| x.match_text == "task-marker"),
        "lowercase prose 'todo' must not fire task-marker"
    );
    let hack = format!(
        "// this is a clever {} that avoids a clone",
        j(&["ha", "ck"])
    );
    assert!(
        !detect_antiprod("src/x.rs", &hack)
            .iter()
            .any(|x| x.match_text == "task-marker"),
        "lowercase prose 'hack' must not fire task-marker"
    );
}

#[test]
fn test_uppercase_marker_still_flagged_after_fix() {
    let j = j;
    let real = format!("// {}: wire this handler", j(&["TO", "DO"]));
    assert!(
        detect_antiprod("src/x.rs", &real)
            .iter()
            .any(|x| x.match_text == "task-marker"),
        "uppercase // TODO: must still be flagged"
    );
    let fixme = format!("    // {} broken under concurrency", j(&["FI", "XM", "E"]));
    assert!(
        detect_antiprod("src/x.rs", &fixme)
            .iter()
            .any(|x| x.match_text == "task-marker"),
        "uppercase // FIXME must still be flagged"
    );
}

#[test]
fn test_lowercase_todo_macro_still_caught_by_stub_macro() {
    let j = j;
    let stub = j(&["to", "do!", "()"]);
    assert!(
        detect_antiprod("src/x.rs", &stub)
            .iter()
            .any(|x| x.match_text == "stub-macro"),
        "todo!() macro must still be caught (idx(r,19), no gap from idx(r,1) fix)"
    );
}
