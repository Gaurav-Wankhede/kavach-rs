//! H4 fail-closed auto-merge DECISION (pure, no IO — fully unit-testable).
//! ALLOW only when EVERY gate holds; any uncertainty denies. This is the
//! highest-blast-radius path in the heal pipeline (merge to main), so the
//! default is DENY and each gate is an explicit AND.
//! SOURCE: roadmap heal.unit.auto-merge-gate (CI green AND 3-witness AND no protected path; default OFF).

/// Path fragments that make a PR INELIGIBLE for auto-merge. A heal touching any
/// of these must go to a human — these are the trust/safety/irreversible
/// surfaces where an autonomous merge is unacceptable. Matched as substrings of
/// each changed file's repo-relative path (case-sensitive, `/`-normalised).
pub(super) const PROTECTED_PATHS: &[&str] = &[
    ".github/",       // CI / workflow definitions (incl. the heal gate itself)
    "migrations/",    // irreversible schema changes
    "/auth",          // authn/authz logic
    "/gnap",          // GNAP auth
    "/paseto",        // token logic
    "/pdp",           // policy decision points
    "/payment",       // money
    "/billing",       // money
    "Cargo.toml",     // dependency surface (supply chain)
    "rust-toolchain", // toolchain pin
    "merge_gate",     // the auto-merge gate must never auto-merge a change to itself
];

/// The auto-merge verdict: `allow` is true ONLY if `reasons` is empty.
pub(super) struct Decision {
    pub allow: bool,
    /// Every failing gate, for the operator/log. Empty ⇒ allowed.
    pub reasons: Vec<String>,
}

/// Pure decision. Fail-closed: starts from "deny" and only an all-pass clears it.
/// `enabled` is the master switch (env, default OFF); `ci_green` + `witness_pass`
/// are the verification gates; `changed` is the PR's changed-file list.
pub(super) fn decide(
    enabled: bool,
    ci_green: bool,
    witness_pass: bool,
    changed: &[String],
) -> Decision {
    let mut reasons = Vec::new();
    if !enabled {
        reasons.push("auto-merge master switch is OFF (set KAVACH_HEAL_AUTOMERGE=1)".to_owned());
    }
    if !ci_green {
        reasons.push("CI is not green".to_owned());
    }
    if !witness_pass {
        reasons.push("3-witness verification did not pass".to_owned());
    }
    if changed.is_empty() {
        // No diff to evaluate ⇒ cannot prove "no protected path" ⇒ deny.
        reasons.push("no changed files reported (cannot prove safety)".to_owned());
    }
    for f in changed {
        if let Some(hit) = protected_hit(f) {
            reasons.push(format!("protected path touched: {f} (matched '{hit}')"));
        }
    }
    Decision {
        allow: reasons.is_empty(),
        reasons,
    }
}

/// First protected fragment that `path` contains, if any.
fn protected_hit(path: &str) -> Option<&'static str> {
    let norm = path.replace('\\', "/");
    PROTECTED_PATHS
        .iter()
        .copied()
        .find(|frag| norm.contains(frag))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn all_pass_clean_diff_allows() {
        let d = decide(true, true, true, &files(&["crates/kavach-ope/src/lib.rs"]));
        assert!(d.allow, "reasons: {:?}", d.reasons);
    }

    #[test]
    fn master_switch_off_denies_even_when_everything_else_passes() {
        let d = decide(false, true, true, &files(&["src/x.rs"]));
        assert!(!d.allow);
        assert!(d.reasons.iter().any(|r| r.contains("master switch")));
    }

    #[test]
    fn ci_red_denies() {
        let d = decide(true, false, true, &files(&["src/x.rs"]));
        assert!(!d.allow && d.reasons.iter().any(|r| r.contains("CI is not green")));
    }

    #[test]
    fn missing_witness_denies() {
        let d = decide(true, true, false, &files(&["src/x.rs"]));
        assert!(!d.allow && d.reasons.iter().any(|r| r.contains("3-witness")));
    }

    #[test]
    fn empty_diff_denies_fail_closed() {
        let d = decide(true, true, true, &[]);
        assert!(!d.allow, "an empty diff must not be auto-mergeable");
    }

    #[test]
    fn protected_path_denies() {
        for p in [
            ".github/workflows/ci.yml",
            "migrations/300_x.sql",
            "crates/services/irongate/src/auth/mod.rs",
            "crates/core/governance/src/pdp/methods.rs",
            "Cargo.toml",
            "crates/kavach-cli/src/cmd/heal/merge_gate.rs",
        ] {
            let d = decide(true, true, true, &files(&[p]));
            assert!(!d.allow, "protected path {p} must deny");
        }
    }

    #[test]
    fn backslash_paths_are_normalised() {
        let d = decide(true, true, true, &files(&[".github\\workflows\\x.yml"]));
        assert!(!d.allow, "windows-style protected path must still match");
    }
}
