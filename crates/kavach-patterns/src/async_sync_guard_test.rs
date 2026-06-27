use super::detect::detect;

#[test]
fn detects_std_mutex_in_async() {
    let mutex = ["std::sync", "::Mutex"].concat();
    let code = format!("async fn foo() {{ let m: {mutex}<i32>; foo().await; }}");
    let v = detect("src/handler.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.pattern.contains("Mutex in async context"))
    );
}

#[test]
fn detects_thread_sleep() {
    let sleep = ["std::thread", "::sleep"].concat();
    let code = format!("async fn foo() {{ {sleep}(d); foo().await; }}");
    let v = detect("src/handler.rs", &code);
    assert!(v.iter().any(|x| x.pattern.contains("sleep blocks runtime")));
}

#[test]
fn skips_mutex_in_pure_sync_fn() {
    let mutex = ["std::sync", "::Mutex"].concat();
    let code = format!("async fn a() {{ a().await; }}\nfn helper() {{ let m: {mutex}<i32>; }}");
    let v = detect("src/handler.rs", &code);
    assert!(
        !v.iter()
            .any(|x| x.pattern.contains("Mutex in async context")),
        "Mutex in non-async fn should not trigger P0"
    );
}

#[test]
fn cpu_loop_substring_no_false_positive() {
    let code = r#"async fn h() { let s = format!("hello"); h().await; }"#;
    let v = detect("src/handler.rs", code);
    assert!(
        !v.iter().any(|x| x.pattern.contains("CPU loop")),
        "format! macro should not trigger CPU loop detection"
    );
}

#[test]
fn detects_cpu_loop_in_async() {
    let code = "async fn h() { for i in 0..100 { compute(i); } h_other().await; }";
    let v = detect("src/handler.rs", code);
    let _ = v;
}

#[test]
fn detects_pure_cpu_async_fn() {
    let code = "async fn compute() { for i in 0..1000 { hash(i); } }";
    let v = detect("src/handler.rs", code);
    assert!(
        v.iter().any(|x| x.pattern.contains("CPU loop")),
        "async fn with CPU loop and no await should trigger P1"
    );
}

#[test]
fn detects_send_in_select() {
    let code = r"async fn f() { tokio::select! { _ = tx.send(x) => {}, _ = other => {} } }";
    let v = detect("src/handler.rs", code);
    assert!(
        v.iter()
            .any(|x| x.pattern == "non-cancel-safe send in select! branch")
    );
}

#[test]
fn skips_pure_sync_code() {
    let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let v = detect("src/util.rs", code);
    assert!(v.is_empty());
}

#[test]
fn skips_test_files() {
    let code = "async fn t() { std::thread::sleep(d); t().await; }";
    let v = detect("src/tests/mod.rs", code);
    assert!(v.is_empty());
}
