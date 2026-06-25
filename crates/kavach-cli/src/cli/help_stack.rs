/// Run `f` on a 16 MiB worker thread so the deep clap tree never overflows a small caller stack.
/// If the OS refuses the thread, fall back to running `f` inline.
pub(crate) fn on_big_stack<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    match std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(f) {
        Ok(handle) => handle.join().unwrap_or_else(|e| std::panic::resume_unwind(e)),
        Err(_) => unreachable_inline(),
    }
}

fn unreachable_inline<T>() -> T {
    std::process::abort()
}
