//! "Presumed-applied" heuristic: a sibling `.applied` marker, or a git commit
//! older than `APPLIED_HEURISTIC_DAYS`. Either signal trips the gate.
use std::path::Path;
use std::process::Command;

pub(super) const APPLIED_HEURISTIC_DAYS: i64 = 7;

/// True when the migration is presumed already-applied (marker or git age).
pub(super) fn is_presumed_applied(target_path: &Path) -> bool {
    let Some(parent) = target_path.parent() else {
        return false;
    };
    let Some(stem) = target_path.file_name() else {
        return false;
    };
    let mut marker = parent.to_path_buf();
    marker.push(format!("{}.applied", stem.to_string_lossy()));
    if marker.exists() {
        return true;
    }
    git_age_days(target_path, parent).is_some_and(|d| d >= APPLIED_HEURISTIC_DAYS)
}

/// Age in whole days of the file's last git commit, or None if unavailable.
fn git_age_days(target_path: &Path, dir: &Path) -> Option<i64> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ct", "--"])
        .arg(target_path)
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let ts: i64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()?;
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
    .unwrap_or(i64::MAX);
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "now >= ts guaranteed by heuristic; subtraction bounded"
    )]
    #[expect(clippy::integer_division, reason = "86_400 is a non-zero constant")]
    Some((now - ts) / 86_400)
}
