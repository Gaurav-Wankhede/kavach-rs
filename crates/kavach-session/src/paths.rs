use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Local;
// BLAKE3: 4x faster than SHA-256 for path slug hashing.

pub(crate) fn home_dir() -> PathBuf {
    std::env::var("HOME").map_or_else(
        |_| std::env::var("USERPROFILE").map_or_else(|_| PathBuf::from("."), PathBuf::from),
        PathBuf::from,
    )
}

// Gated to `all(test, unix)` to match its ONLY caller — the `#[cfg(unix)]`
// fault-injection test in enforcement.rs that needs an unwritable state dir
// (chmod 0o000 is a Unix-only construct). On Windows the caller compiles out,
// so a bare `#[cfg(test)]` here would leave `set_test_state_dir` dead-coded and
// trip the workspace's denied `dead_code` lint under `-D warnings`.
// Thread-local override (no `unsafe` env mutation; workspace is forbid(unsafe)).
// Live under own-tests OR a downstream `test-support` consumer (engine spool glue).
#[cfg(any(test, feature = "test-support"))]
thread_local! {
    static TEST_STATE_DIR: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// Test-only: redirect `state_dir()` for the current thread (`None` clears).
///
/// Used by fault-injection tests and downstream `test-support` consumers that
/// need a temp state dir without touching real session state or process env.
#[cfg(any(test, feature = "test-support"))]
pub fn set_test_state_dir(dir: Option<PathBuf>) {
    TEST_STATE_DIR.with(|c| *c.borrow_mut() = dir);
}

/// Shared state directory (XDG on Linux, Library on macOS, LOCALAPPDATA on Windows).
#[must_use]
pub fn state_dir() -> PathBuf {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(d) = TEST_STATE_DIR.with(|c| c.borrow().clone()) {
        return d;
    }
    if cfg!(target_os = "macos") {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("SharedAI")
            .join("state")
    } else if cfg!(target_os = "windows") {
        let base = std::env::var("LOCALAPPDATA").map_or_else(|_| home_dir(), PathBuf::from);
        base.join("SharedAI").join("state")
    } else {
        let xdg = std::env::var("XDG_DATA_HOME")
            .map_or_else(|_| home_dir().join(".local").join("share"), PathBuf::from);
        xdg.join("shared-ai").join("state")
    }
}

/// STM (short-term memory) path -- same as state dir for session files.
#[must_use]
pub fn stm_path() -> PathBuf {
    state_dir()
}

/// Compute a stable 16-character hex digest of the canonical working directory.
/// Used to scope session state files per terminal/project so state never
/// leaks across concurrent Claude Code sessions running in different cwds.
fn workdir_slug() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let canonical = fs::canonicalize(&cwd).unwrap_or(cwd);
    slug_from_path(&canonical)
}

fn slug_from_path(path: &Path) -> String {
    slug_from_bytes(path.to_string_lossy().as_bytes())
}

/// Path to the session state file, scoped per working directory.
///
/// Each terminal/project gets an isolated file so state never leaks across
/// concurrent Claude Code sessions. Migrates legacy shared files on first access
/// into the scoped file for the current cwd.
///
/// When `session_id` is non-empty (Cursor `conversation_id`, Claude Code session),
/// the path also includes a session slug so concurrent conversations in ONE
/// workdir do not clobber each other's INI cache when RPC is down.
#[must_use]
pub fn state_path_for(session_id: &str) -> PathBuf {
    let slug = workdir_slug();
    if session_id.is_empty() {
        return state_path_for_workdir_slug(&slug);
    }
    let sess = slug_from_bytes(session_id.as_bytes());
    stm_path().join(format!("session-state-{slug}-{sess}.kavach"))
}

fn state_path_for_workdir_slug(slug: &str) -> PathBuf {
    let target = stm_path().join(format!("session-state-{slug}.kavach"));
    if target.exists() {
        return target;
    }
    for name in [
        "session-state.kavach",
        "session-state.ini",
        "session-state.toon",
    ] {
        let legacy = stm_path().join(name);
        if legacy.exists() && fs::rename(&legacy, &target).is_ok() {
            return target;
        }
    }
    target
}

fn slug_from_bytes(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    let mut hex = String::with_capacity(16);
    for byte in hash.as_bytes().iter().take(8) {
        std::fmt::Write::write_fmt(&mut hex, format_args!("{byte:02x}")).ok();
    }
    hex
}

/// Path to the session state file, scoped per working directory (no conversation id).
///
/// Prefer [`state_path_for`] when a session id is known.
#[must_use]
pub fn state_path() -> PathBuf {
    state_path_for_workdir_slug(&workdir_slug())
}

/// Canonicalize a user-supplied path for filesystem comparison.
///
/// Compares filesystem identity, not byte equality. Falls back to
/// absolute-but-unresolved when the file doesn't yet exist (new file workflow),
/// then to the raw input as last resort. Used by both `kavach phase iteration-start`
/// (storage) and the `pre_write` gate (comparison) so relative and absolute
/// spellings of the same file resolve identically.
#[must_use]
pub fn canonicalize_iteration_path(input: &str) -> String {
    if let Ok(p) = fs::canonicalize(input) {
        return p.to_string_lossy().into_owned();
    }
    if let Ok(cwd) = std::env::current_dir() {
        let joined: PathBuf = if Path::new(input).is_absolute() {
            PathBuf::from(input)
        } else {
            cwd.join(input)
        };
        return joined.to_string_lossy().into_owned();
    }
    input.to_owned()
}

pub(crate) fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Detect project slug from the kavach-rpc server using path lookup.
/// Falls back to directory name if the server is not running or no match.
pub(crate) fn detect_project() -> String {
    let Ok(cwd) = std::env::current_dir() else {
        // F3: a cwd error makes EVERY kanban RPC key on the phantom "unknown" project,
        // silently returning empty boards. Fail LOUD on stderr (never stdout — that is
        // the hook verdict channel) so the degradation is visible, not silent.
        warn_degraded(
            "current_dir() failed — project resolves to \"unknown\"; \
             all kanban/decision reads will be empty this session.",
        );
        return "unknown".into();
    };
    let cwd_str = cwd.to_string_lossy();

    // Try kavach-rpc lookup first (authoritative source via SurrealDB)
    let params = serde_json::json!({"path": cwd_str.as_ref()});
    if let Ok(project) = kavach_rpc::client::call::<_, kavach_surreal::Project>(
        "projects.find_by_path",
        Some(params),
    ) {
        return project.slug;
    }

    // Fallback: directory name, normalized to a slug so downstream RPC calls
    // (roadmap.next_open_task, roadmap.entry_status) hit the same identifier
    // the kanban tables key on. Without this, "Nicole Carpenter" workdirs
    // silently return None from every kanban RPC and the stop-hook
    // AUTO_CONTINUE branch dies (observed 2026-05).
    // SOURCE: same contract violation class fixed earlier in cmd/harness_loop.rs
    // RESEARCH: https://docs.rs/slug (canonical algorithm) — std-only impl below
    let slug = cwd.file_name().map_or_else(
        || "unknown".into(),
        |n| slugify_project(&n.to_string_lossy()),
    );
    // F3: RPC missed → slug is a guess; on slug-drift every board read is silently
    // empty (cf. fix-shortform-platform-slug). Warn LOUD on stderr.
    warn_degraded(&format!(
        "project not found via RPC for `{cwd_str}` — falling back to dir-name slug \
         `{slug}`. If kanban reads come back empty, the registered project slug differs; \
         register the path or run from the canonical dir."
    ));
    slug
}

/// Emit an anti-amnesia degradation warning to stderr (never stdout — that is the
/// hook verdict channel). Matches the crate's `eprintln!` diagnostic precedent.
fn warn_degraded(msg: &str) {
    #[expect(clippy::print_stderr, reason = "anti-amnesia degradation warning to audit trail")]
    {
        eprintln!("kavach: WARN {msg}");
    }
}

/// Normalize a directory name to a kanban-compatible slug:
/// ASCII alphanumerics lowercased, all other characters collapsed to '-',
/// leading/trailing '-' stripped, empty result -> "unknown".
/// "Nicole Carpenter" -> "nicole-carpenter"
/// "`iron_will_v2`" -> "iron-will-v2"
/// "  My Project!  " -> "my-project"
fn slugify_project(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_dash = true;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "unknown".into()
    } else {
        out
    }
}

#[cfg(test)]
mod slug_tests {
    use super::slugify_project;

    #[test]
    fn lowercases_and_dashes() {
        assert_eq!(slugify_project("Nicole Carpenter"), "nicole-carpenter");
    }

    #[test]
    fn underscores_become_dashes() {
        assert_eq!(slugify_project("iron_will_v2"), "iron-will-v2");
    }

    #[test]
    fn punctuation_collapses() {
        assert_eq!(slugify_project("  My Project!  "), "my-project");
    }

    #[test]
    fn already_slug_passes_through() {
        assert_eq!(slugify_project("kavach-rs"), "kavach-rs");
    }

    #[test]
    fn empty_becomes_unknown() {
        assert_eq!(slugify_project(""), "unknown");
    }
}

pub(crate) fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub(crate) fn now_datetime() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_stable_for_same_path() {
        let p = Path::new("/tmp/kavach-test-a");
        assert_eq!(slug_from_path(p), slug_from_path(p));
    }

    #[test]
    fn slug_differs_across_paths() {
        let a = slug_from_path(Path::new("/tmp/project-a"));
        let b = slug_from_path(Path::new("/tmp/project-b"));
        assert_ne!(a, b);
    }

    #[test]
    fn slug_is_16_hex_chars() {
        let s = slug_from_path(Path::new("/any/path"));
        assert_eq!(s.len(), 16);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn state_path_contains_slug() {
        let p = state_path();
        let name = p
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().to_string());
        assert!(name.starts_with("session-state-"));
        assert!(name.ends_with(".kavach"));
    }

    #[test]
    fn state_path_for_session_includes_session_slug() {
        let a = state_path_for("conv-a");
        let b = state_path_for("conv-b");
        assert_ne!(a, b);
        let name = a
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().to_string());
        assert!(name.matches('-').count() >= 2, "workdir + session slugs: {name}");
    }
}
