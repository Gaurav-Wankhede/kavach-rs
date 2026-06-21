//! Version-scheme-aware stability primitive for the research-refreshed patterns.
//!
//! Decides whether a version bump crosses a BREAKING boundary, inferring the
//! scheme from the version itself — never a hardcoded per-package rule, so it is
//! language/tech-stack agnostic. Boundary rule (`SemVer` §4 "Major version
//! zero"): for a 0.x release the MINOR field is the breaking axis (0.7 → 0.8 may
//! break anything), so a young framework like Dioxus 0.7.9 treats 0.7 → 0.8 as
//! breaking but 0.7.9 → 0.7.10 as safe. For 1.x+ the MAJOR field is the breaking
//! axis (standard `SemVer`). SOURCE: <https://semver.org/#spec-item-4>.

/// A parsed `MAJOR.MINOR.PATCH` version. Missing trailing fields default to 0
/// (`"1"` → 1.0.0, `"0.7"` → 0.7.0), and a leading `v`/`^`/`~`/`=` is tolerated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Version {
    /// Major field.
    pub major: u64,
    /// Minor field.
    pub minor: u64,
    /// Patch field.
    pub patch: u64,
}

impl Version {
    /// Parse a version string, tolerating a leading range operator and a
    /// pre-release/build suffix (`-rc.1`, `+meta`), which are ignored for the
    /// breaking-boundary decision. Returns `None` if no numeric major is found.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim().trim_start_matches(['v', 'V', '^', '~', '=', ' ']);
        // Drop a pre-release / build-metadata suffix: keep up to the first
        // char that is neither a digit nor a dot.
        let core_end = trimmed
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(trimmed.len());
        let core = trimmed.get(..core_end)?;
        let mut it = core.split('.');
        let major = it.next()?.parse::<u64>().ok()?;
        let minor = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    /// True when this version is in the 0.x "major version zero" regime, where
    /// the minor field is the breaking axis.
    #[must_use]
    pub const fn is_zero_ver(self) -> bool {
        self.major == 0
    }
}

/// Verdict for a version transition from `from` to `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BumpKind {
    /// Crosses the scheme's breaking boundary — adopt only after soak/research.
    Breaking,
    /// Within the compatible range — safe to adopt.
    Compatible,
    /// `to` is not strictly newer than `from` (downgrade or no-op).
    NoForwardChange,
}

/// Classify the bump from `from` to `to` using the version-scheme-aware rule.
///
/// - 0.x: a change in MAJOR or MINOR is `Breaking`; only a PATCH-only change is
///   `Compatible`.
/// - 1.x+: a change in MAJOR is `Breaking`; MINOR/PATCH changes are
///   `Compatible`.
///
/// A scheme jump out of 0.x (e.g. 0.9 → 1.0) is always `Breaking` — the 1.0
/// release is the stabilization boundary.
#[must_use]
pub fn classify_bump(from: Version, to: Version) -> BumpKind {
    if !is_strictly_newer(from, to) {
        return BumpKind::NoForwardChange;
    }
    // Leaving the 0.x regime (0.x → 1.0+) is the stabilization boundary.
    if from.is_zero_ver() != to.is_zero_ver() {
        return BumpKind::Breaking;
    }
    // 0.x: minor is the breaking axis (major is always 0 here). 1.x+: major is
    // the breaking axis. In both cases a difference on that axis ⇒ Breaking.
    let axis_changed = if to.is_zero_ver() {
        to.minor != from.minor
    } else {
        to.major != from.major
    };
    if axis_changed {
        BumpKind::Breaking
    } else {
        BumpKind::Compatible
    }
}

/// Lexicographic (major, minor, patch) strictly-greater comparison.
fn is_strictly_newer(from: Version, to: Version) -> bool {
    (to.major, to.minor, to.patch) > (from.major, from.minor, from.patch)
}

#[cfg(test)]
#[path = "stability_test.rs"]
mod stability_test;
