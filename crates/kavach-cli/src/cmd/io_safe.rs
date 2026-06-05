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

use std::io::{self, Write};

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
}
