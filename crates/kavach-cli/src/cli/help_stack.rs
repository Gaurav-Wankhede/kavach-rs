/// Run `f` on a 16 MiB worker thread so the deep clap tree never overflows a small caller stack.
pub(crate) fn on_big_stack<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(f)
        .expect("spawn help worker thread");
    handle
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e))
}
