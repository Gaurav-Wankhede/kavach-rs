/// Run `f` on a 16 MiB worker thread so the deep clap tree never overflows a small caller stack.
pub(crate) fn on_big_stack<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(f)
        .and_then(std::thread::JoinHandle::join)
        .unwrap_or_else(|e| std::panic::resume_unwind(e.into()))
}
