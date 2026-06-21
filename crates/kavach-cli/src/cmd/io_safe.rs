// ARCH: see kavach db get --category decision --key arch.decision.silent_io_guard_shipped
//! Honest IO helpers for CLI handlers.
//!
//! Each handler returning i32 must propagate stdout/stderr write failures
//! (broken pipe, full disk, EIO) instead of swallowing them with the band-aid
//! `let _ = writeln!(...)` pattern blocked by `silent_io_guard`.
//!
//! USAGE in a handler returning i32:
//! ```ignore
//! if let Err(e) = print_or_exit("hello") {
//!     return into_exit_code(e);
//! }
//! ```

use std::io::{self, IsTerminal, Read, Write};

/// POSIX sysexits.h `EX_IOERR` — input/output error.
pub(crate) const EX_IOERR: i32 = 74;

/// IO failure surfaced to the caller. Carries the source `io::Error` so the
/// caller can log it before exiting — no error context is discarded.
#[derive(Debug)]
pub(crate) struct IoExit {
    pub source: io::Error,
    pub code: i32,
}

impl From<io::Error> for IoExit {
    fn from(source: io::Error) -> Self {
        Self {
            source,
            code: EX_IOERR,
        }
    }
}

pub(crate) fn print_or_exit(line: &str) -> Result<(), IoExit> {
    let mut h = io::stdout().lock();
    h.write_all(line.as_bytes())?;
    h.write_all(b"\n")?;
    Ok(())
}

pub(crate) fn ewrite_or_exit(line: &str) -> Result<(), IoExit> {
    let mut h = io::stderr().lock();
    h.write_all(line.as_bytes())?;
    h.write_all(b"\n")?;
    Ok(())
}

/// Print `error: <msg>` to stderr and return the process exit code (1, or the
/// IO failure's code if even stderr can't be written). The shared error path
/// for `db` subcommand handlers.
pub(crate) fn err_exit(msg: &str) -> i32 {
    let line = format!("error: {msg}");
    match ewrite_or_exit(&line) {
        Ok(()) => 1,
        Err(io) => into_exit_code(io),
    }
}

/// Print a prompt (no trailing newline) and read one trimmed line from stdin.
/// Used by destructive commands to collect the typed confirmation phrase.
/// Returns the trimmed input, or an `IoExit` if stdout/stdin fails.
pub(crate) fn prompt_line_or_exit(prompt: &str) -> Result<String, IoExit> {
    let mut out = io::stdout().lock();
    out.write_all(prompt.as_bytes())?;
    out.flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\n', '\r']).to_owned())
}

/// Read the entry body from stdin when `--content` was omitted.
///
/// Returns `Ok(Some(body))` when stdin is a pipe/redirect (non-TTY) and carried
/// bytes — the documented `--content reads from stdin if omitted` contract.
/// Returns `Ok(None)` when stdin is an interactive terminal (no piped input) OR
/// the piped input was empty, so the caller can fall back to the title-only
/// warning for a human who simply forgot the flag. Propagates an `IoExit` on a
/// real stdin read failure (EIO, etc.) — never swallows it.
///
/// `IsTerminal` (std, Rust 1.70+) is the maintained successor to the
/// now-unmaintained `atty` crate — no external dependency, no `unsafe`.
/// SOURCE: <https://doc.rust-lang.org/std/io/trait.IsTerminal.html>
pub(crate) fn read_stdin_body_or_exit() -> Result<Option<String>, IoExit> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        // Interactive TTY with no redirect: nothing was piped in.
        return Ok(None);
    }
    let mut body = String::new();
    stdin.lock().read_to_string(&mut body)?;
    Ok(classify_stdin_body(body))
}

/// Pure body-classification seam (testable without a real stdin/TTY): an empty
/// piped body is treated as "no body" so the caller falls back to the title-only
/// warning exactly as an interactive TTY would — a redirect from `/dev/null`
/// must not silently overwrite content with an empty string.
fn classify_stdin_body(body: String) -> Option<String> {
    if body.is_empty() { None } else { Some(body) }
}

/// Convert an `IoExit` into an exit code after best-effort logging.
/// `.ok()` (NOT `let _ =`) is the documented way to discard the final stderr
/// write — by here the primary stream already failed; nowhere to report.
#[expect(
    clippy::needless_pass_by_value,
    reason = "terminal error sink called at 500+ sites by value; taking IoExit by-value keeps every call `into_exit_code(io_err)` rather than churning the whole crate to `&io_err`"
)]
pub(crate) fn into_exit_code(e: IoExit) -> i32 {
    let msg = format!("kavach: io failure ({}) — exiting {}\n", e.source, e.code);
    io::stderr().lock().write_all(msg.as_bytes()).ok();
    e.code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_smoke() {
        assert!(print_or_exit("test").is_ok());
    }

    #[test]
    fn ewrite_smoke() {
        assert!(ewrite_or_exit("test").is_ok());
    }

    #[test]
    fn ioexit_carries_source() {
        let e = io::Error::new(io::ErrorKind::BrokenPipe, "pipe broke");
        let exit: IoExit = e.into();
        assert_eq!(exit.code, EX_IOERR);
        assert_eq!(exit.source.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn stdin_body_nonempty_is_some() {
        // A piped/redirected body supplies the entry content (the documented
        // `--content reads from stdin if omitted` contract).
        assert_eq!(
            classify_stdin_body("hello body".to_owned()),
            Some("hello body".to_owned())
        );
    }

    #[test]
    fn stdin_body_empty_is_none() {
        // An empty pipe (e.g. `< /dev/null`) must NOT overwrite content with ""
        // — it falls back to the title-only warning like an interactive TTY.
        assert_eq!(classify_stdin_body(String::new()), None);
    }

    #[test]
    fn stdin_body_preserves_whitespace_and_newlines() {
        // The body is taken verbatim — leading/trailing whitespace and embedded
        // newlines (a multi-line plan doc) are content, not noise to trim.
        let doc = "  line1\nline2\n".to_owned();
        assert_eq!(classify_stdin_body(doc.clone()), Some(doc));
    }
}
