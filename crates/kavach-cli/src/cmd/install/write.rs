//! Safe install-write: substitute the absolute binary path, then write the
//! tool's OFFICIAL config — idempotent, back-up-before-overwrite, never clobber a
//! user's existing hooks silently. `--dry-run` prints the would-be action only.
//!
//! Loophole posture (config installer, not a network/auth path):
//! - replay  : re-running is a no-op when the file is byte-identical (`Outcome::Unchanged`).
//! - failure : the prior file is copied to `<path>.kavach.bak` BEFORE any overwrite.
//! - malformed: the template is embedded (compile-time), so the SOURCE is trusted;
//!   an existing on-disk file we'd overwrite is preserved via the backup.
//! - boundary: a missing parent dir is created; an empty/absent file is a plain create.

use std::fs;
use std::path::Path;

/// What an install attempt did, for honest reporting.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// No file existed; wrote a fresh config.
    Created,
    /// File existed with DIFFERENT content; backed it up, then overwrote.
    Overwrote,
    /// File already byte-identical; did nothing (idempotent re-run).
    Unchanged,
    /// `--dry-run`: nothing written; carries the action that WOULD occur.
    DryRun(&'static str),
}

/// Substitute the bare `kavach` invocation in a template with the absolute path
/// to THIS binary, so the installed hook resolves regardless of `$PATH`.
pub(super) fn render(template: &str, binary: &Path) -> String {
    let abs = binary.display().to_string();
    // Templates invoke `kavach gates ...`; pin every call to the absolute binary.
    template.replace("kavach gates ", &format!("{abs} gates "))
}

/// Write `body` to `path`, applying the safety policy. Returns the outcome or an
/// IO error (fail-closed: a write failure is propagated, never swallowed).
pub(super) fn install(path: &Path, body: &str, dry_run: bool) -> std::io::Result<Outcome> {
    let existing = match fs::read_to_string(path) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };

    // Idempotent: identical content is a guaranteed no-op on every re-run.
    if existing.as_deref() == Some(body) {
        return Ok(Outcome::Unchanged);
    }

    if dry_run {
        return Ok(Outcome::DryRun(if existing.is_some() {
            "would back up + overwrite"
        } else {
            "would create"
        }));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // failure lens: copy the prior file aside BEFORE overwriting, so a botched
    // install never destroys the user's hand-edited config.
    let overwrote = existing.is_some();
    if overwrote {
        let mut bak = path.as_os_str().to_owned();
        bak.push(".kavach.bak");
        fs::write(Path::new(&bak), existing.unwrap_or_default())?;
    }

    fs::write(path, body)?;
    Ok(if overwrote {
        Outcome::Overwrote
    } else {
        Outcome::Created
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_pins_absolute_binary() {
        let out = render(
            "kavach gates stop --hook",
            Path::new("/usr/local/bin/kavach"),
        );
        assert_eq!(out, "/usr/local/bin/kavach gates stop --hook");
    }

    #[test]
    fn create_then_idempotent_rerun() {
        let dir = std::env::temp_dir().join(format!("kvinst-{}", std::process::id()));
        let p = dir.join("cfg.json");
        fs::remove_dir_all(&dir).ok();
        assert_eq!(install(&p, "A", false).unwrap(), Outcome::Created);
        // Re-run with identical body: no-op, no backup spawned.
        assert_eq!(install(&p, "A", false).unwrap(), Outcome::Unchanged);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn overwrite_backs_up_prior() {
        let dir = std::env::temp_dir().join(format!("kvinst-ow-{}", std::process::id()));
        let p = dir.join("cfg.json");
        fs::remove_dir_all(&dir).ok();
        install(&p, "OLD", false).unwrap();
        assert_eq!(install(&p, "NEW", false).unwrap(), Outcome::Overwrote);
        let bak = fs::read_to_string(dir.join("cfg.json.kavach.bak")).unwrap();
        assert_eq!(bak, "OLD", "prior content must survive in the backup");
        assert_eq!(fs::read_to_string(&p).unwrap(), "NEW");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dry_run_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("kvinst-dry-{}", std::process::id()));
        let p = dir.join("cfg.json");
        fs::remove_dir_all(&dir).ok();
        assert!(matches!(
            install(&p, "X", true).unwrap(),
            Outcome::DryRun(_)
        ));
        assert!(!p.exists(), "dry-run must not create the file");
        fs::remove_dir_all(&dir).ok();
    }
}
