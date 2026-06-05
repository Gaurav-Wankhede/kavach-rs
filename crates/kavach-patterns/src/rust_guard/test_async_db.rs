//! Async/perf + silent-discard detector tests (lock-across-await, linear search,
//! string-alloc-in-loop, chatty awaits, silent DB/await result discard).
use crate::rust_guard::{RustSeverity, detect};

#[test]
fn p0_lock_across_await() {
    let code = "let guard = mutex.lock().await;\ndo_work(guard).await;";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern == "lock held across await"));
}

#[test]
fn p0_linear_search_in_loop() {
    let code = "for item in items {\n    if seen.contains(&item) { skip(); }\n}";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern == "linear search in loop"));
}

#[test]
fn p1_string_alloc_in_loop() {
    let code = "for name in names {\n    let msg = format!(\"hello {}\", name);\n}";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern == "string allocation in loop"));
}

#[test]
fn p1_chatty_sequential_awaits() {
    let code = "let a = svc.get_user().await?;\nlet b = svc.get_wallet().await?;\nlet c = svc.get_missions().await?;";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern == "chatty sequential awaits"));
}

#[test]
fn p0_silent_db_discard() {
    let code = "let _result = sqlx::query(\"DELETE FROM roles\").execute(&pool).await;";
    let v = detect("src/lib.rs", code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "silent DB result discard")
    );
}

#[test]
fn p0_silent_db_discard_underscore_var() {
    let code = "let _res = conn.execute(\"UPDATE users SET active = false\").await;";
    let v = detect("src/lib.rs", code);
    assert!(v.iter().any(|x| x.pattern == "silent DB result discard"));
}

#[test]
fn p0_silent_await_discard() {
    let code = "let _ = service.send_notification().await;";
    let v = detect("src/lib.rs", code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "silent await discard")
    );
}

#[test]
fn ok_when_result_used() {
    let code = "let result = sqlx::query(\"SELECT 1\").fetch_one(&pool).await?;";
    let v = detect("src/lib.rs", code);
    assert!(!v.iter().any(|x| x.pattern == "silent DB result discard"));
}

#[test]
fn ok_when_result_checked() {
    let code = "let rows = sqlx::query(\"DELETE FROM old_data\").execute(&pool).await;";
    let v = detect("src/lib.rs", code);
    assert!(!v.iter().any(|x| x.pattern == "silent DB result discard"));
}
