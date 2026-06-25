/// Run `f` on a 16 MiB worker thread so building the deep clap tree never
/// overflows a small caller stack (e.g. nextest's 2 MiB test threads).
pub(super) fn on_big_stack<F>(f: F) -> String
where
    F: FnOnce() -> String + Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(f)
        .ok()
        .and_then(|h| h.join().ok())
        .unwrap_or_default()
}
