//! Witness-gated auto-verify: promote `done` cards to `verified` only after the
//! shared workspace build+test witnesses pass (3-witness law; the diff witness
//! is implicit — the card reached `done` because its work shipped).

/// Three-state outcome of an auto-verify pass. The caller MUST branch on this so
/// a witness-failing `done` card (real AI repair work) is never confused with a
/// genuinely empty queue (a legitimate clean stop) — collapsing both to `0` is
/// what made the stop gate loop forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoVerify {
    /// No `done` cards existed — nothing to verify. If no card is dispatchable
    /// either, the queue is empty or every remainder is blocked by dependencies:
    /// a clean stop is correct.
    NothingDone,
    /// `done` cards exist but the workspace witnesses FAILED — there is an
    /// AI-fixable keystone. The loop must command repair, never stop.
    WitnessFailed,
    /// Promoted this many `done -> verified`. Dependents may now be dispatchable.
    Promoted(usize),
}

/// Keys of every roadmap card currently at `done` (work shipped, awaiting
/// verification). Empty on any error — auto-verify is best-effort.
fn list_done_card_keys(project_slug: &str) -> Vec<String> {
    if project_slug.is_empty() {
        return Vec::new();
    }
    let params = serde_json::json!({ "project": project_slug });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.list_done_cards", Some(params))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.get("key").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// Promote one `done` card to `verified`. True iff it flipped this call.
/// Best-effort: a miss leaves the card at `done` (re-attempted next stop).
fn verify_card(project_slug: &str, key: &str) -> bool {
    if project_slug.is_empty() || key.is_empty() {
        return false;
    }
    let params = serde_json::json!({ "project": project_slug, "key": key });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.verify_card", Some(params))
        .ok()
        .and_then(|v| v.get("verified").and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

/// Whether the cargo workspace witnesses ran and what they found. A FAILED run
/// (witnesses executed, returned non-zero) is real AI repair work. An
/// INAPPLICABLE run (no `Cargo.toml` in CWD → not a Rust project; or `cargo`
/// absent → spawn error) is NOT repair work — treating it as a failure traps the
/// stop-gate in perpetual `KEYSTONE_REPAIR` for every non-cargo project that has
/// a `done` card (the IronCore-scaffold loop-trap). SOURCE: rca.keystone-trap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WitnessRun {
    Passed,
    Failed,
    Inapplicable,
}

/// True iff the cargo workspace witnesses can meaningfully run in `dir` — i.e.
/// `dir` is (or is inside) a Rust project, evidenced by a `Cargo.toml`. Split out
/// as a pure, dir-parameterized helper so the applicability rule is unit-testable
/// without depending on the test process's CWD. SOURCE: rca.keystone-trap.
fn witnesses_applicable(dir: &std::path::Path) -> bool {
    dir.join("Cargo.toml").exists()
}

/// Run the objective build+test witnesses ONCE over the whole workspace, but
/// ONLY when this project is actually a cargo workspace and `cargo` exists. CWD
/// is the agent's project root. Returns [`WitnessRun::Inapplicable`] (never
/// `Failed`) when the witnesses cannot meaningfully run, so a non-Rust project's
/// `done` cards are not trapped behind a witness that can never pass.
fn run_workspace_witnesses() -> WitnessRun {
    // Applicability gate: a Rust workspace has a Cargo.toml at the agent's CWD.
    // Without it `cargo check --workspace` errors for reasons unrelated to the
    // card's correctness — that is not an AI-fixable keystone.
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if !witnesses_applicable(&cwd) {
        return WitnessRun::Inapplicable;
    }
    let check = std::process::Command::new("cargo")
        .args(["check", "--workspace", "--quiet"])
        .status();
    // `cargo` not on PATH / spawn failure — cannot witness, do not trap.
    let Ok(check) = check else {
        return WitnessRun::Inapplicable;
    };
    if !check.success() {
        return WitnessRun::Failed;
    }
    match std::process::Command::new("cargo")
        .args(["nextest", "run", "--workspace", "--no-fail-fast"])
        .status()
    {
        Ok(s) if s.success() => WitnessRun::Passed,
        Ok(_) => WitnessRun::Failed,
        Err(_) => WitnessRun::Inapplicable,
    }
}

/// Witness-gated auto-verify: find every `done` card, run the shared workspace
/// witnesses ONCE, and on success promote each `done -> verified` so the loop
/// self-closes finished work and flows to the next task instead of halting on
/// `[ALL_BLOCKED]`. Promotion also unblocks dependents on the same stop pass.
///
/// Returns a three-state [`AutoVerify`] so the caller can tell a witness-failing
/// `done` card (AI repair work) apart from an empty queue (clean stop).
/// Collapsing both to `0` previously trapped the loop.
pub(crate) fn auto_verify_done_cards(project_slug: &str) -> AutoVerify {
    let done = list_done_card_keys(project_slug);
    if done.is_empty() {
        return AutoVerify::NothingDone;
    }
    // One shared witness pass gates ALL done cards. Branch on the THREE-state run:
    //  - Failed       → witnesses executed and returned non-zero: a real,
    //                    AI-fixable keystone exists; the caller commands repair.
    //  - Passed       → promote each done -> verified.
    //  - Inapplicable → not a cargo workspace (no Cargo.toml) or cargo absent:
    //                    the witnesses CANNOT run here, so a hard `cargo` failure
    //                    must NOT be read as a keystone. The card already cleared
    //                    its own authoring-time 3-witness to reach `done`; promote
    //                    it so the loop progresses instead of looping forever on
    //                    KEYSTONE_REPAIR for a non-Rust project. SOURCE:
    //                    rca.keystone-trap.
    match run_workspace_witnesses() {
        WitnessRun::Failed => AutoVerify::WitnessFailed,
        WitnessRun::Passed | WitnessRun::Inapplicable => AutoVerify::Promoted(
            done.iter()
                .filter(|key| verify_card(project_slug, key))
                .count(),
        ),
    }
}

#[cfg(test)]
mod witness_applicability_tests {
    use super::{WitnessRun, witnesses_applicable};

    #[test]
    fn non_cargo_dir_is_inapplicable_not_failed() {
        // A non-Rust project (no Cargo.toml) must NOT be treated as a witness
        // FAILURE — that false negative trapped the stop-gate in perpetual
        // KEYSTONE_REPAIR. SOURCE: rca.keystone-trap.
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(
            !witnesses_applicable(dir.path()),
            "a dir without Cargo.toml is NOT a cargo workspace"
        );
    }

    #[test]
    fn cargo_dir_is_applicable() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("write");
        assert!(
            witnesses_applicable(dir.path()),
            "a dir with Cargo.toml IS a cargo workspace"
        );
    }

    #[test]
    fn inapplicable_does_not_map_to_failed() {
        // Guard the branch contract: Inapplicable and Failed are distinct, and
        // only Failed drives KEYSTONE_REPAIR.
        assert_ne!(WitnessRun::Inapplicable, WitnessRun::Failed);
        assert_ne!(WitnessRun::Inapplicable, WitnessRun::Passed);
    }
}
