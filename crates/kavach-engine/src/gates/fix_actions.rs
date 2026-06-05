/// Maps violation codes to (action, instruction) pairs for fix guidance.
pub(crate) fn fix_action(code: &str, match_text: &str) -> (&'static str, &'static str) {
    match code {
        "MOCK_DATA" => (
            "REPLACE with real data source",
            "Replace hardcoded data with a database query or API call",
        ),
        "PROD_LEAK" => prod_leak_action(match_text),
        "ERROR_BLIND" => error_blind_action(match_text),
        "TYPE_LOOSE" => (
            "REMOVE type suppression",
            "Fix the underlying type error instead of suppressing it",
        ),
        _ => (
            "FIX the violation",
            "Review and correct the flagged pattern",
        ),
    }
}

fn prod_leak_action(match_text: &str) -> (&'static str, &'static str) {
    match match_text {
        "console.log" | "print()" | "fmt.Print" | "System.out" | "print-macro" | "debug-macro" => (
            "REMOVE or replace with structured logging",
            "Replace debug output with tracing::info! or tracing::debug!",
        ),
        "task-marker" => (
            "IMPLEMENT the marked task now",
            "Implement the stub inline — task markers are banned in production code",
        ),
        "stub-macro" => (
            "REPLACE stub with complete implementation",
            "Implement the function body — stubs not allowed in production",
        ),
        "localhost" => (
            "REPLACE with config variable",
            "Read host/port from environment variable or config file",
        ),
        "eval()" | "new Function()" | "eval/exec" => (
            "REMOVE code injection vector",
            "Remove eval/exec — no dynamic code execution in production",
        ),
        "dangerousSetHTML" | "innerHTML =" | "document.write" => (
            "REMOVE XSS vector",
            "Render with JSX or set textContent — raw HTML injection enables XSS",
        ),
        "unsafe block" => (
            "ADD // SAFETY: comment or remove unsafe",
            "Add // SAFETY: comment explaining the invariant. Invoke /rust for unsafe code review",
        ),
        "rs-abort-macro" | "proc-exit" => (
            "RETURN Result instead of aborting",
            "Return Err via ? — propagate errors instead of aborting. Invoke /error",
        ),
        _ => (
            "REMOVE or replace with production-safe alternative",
            "Eliminate debug/unsafe pattern from production code",
        ),
    }
}

fn error_blind_action(match_text: &str) -> (&'static str, &'static str) {
    match match_text {
        "unwrap-in-handler" => (
            "REPLACE with ? operator",
            "Replace with ? or match — panics in handlers crash the process",
        ),
        "excessive-clone" => (
            "REDUCE clones — borrow instead",
            "Borrow instead of cloning — pass &T or use Cow<T>. Invoke /rust",
        ),
        _ => (
            "ADD proper error handling",
            "Handle the error with typed Result or explicit catch block",
        ),
    }
}
