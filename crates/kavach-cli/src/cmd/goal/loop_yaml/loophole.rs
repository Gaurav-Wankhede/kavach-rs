// Meta-Harness Loophole Loop data model (extends loop_yaml). The per-iteration
// YAML artifact emitted to a /tmp working dir so each round PRECISELY targets one
// unit of work. The six attack lenses mirror CLAUDE.md §loophole_self_interrogation.
//
// YAML (not Markdown) for the loop pipeline: deterministic, diffable, machine-
// targetable — SOURCE research.yaml-vs-markdown-meta-harness-loop ·
// decision.meta.loophole-loop-extends-goal-yaml.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One adversarial attack lens. Each names a failure mode the happy path never
/// exercises; the sweep runs every lens over the target and records what breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Lens {
    /// Two actors at once → TOCTOU / lost-update / double-claim.
    Concurrency,
    /// Process dies mid-op → orphaned lock / half-write / leaked task.
    Failure,
    /// null / huge / wrong-type / hostile input → panic / injection.
    Malformed,
    /// Caller without rights → missing check / confused-deputy / IDOR.
    Authz,
    /// Same request twice → non-idempotent mutation.
    Replay,
    /// empty / max / negative / off-by-one.
    Boundary,
}

impl Lens {
    /// Every lens, in the canonical order the sweep runs them.
    pub(crate) const ALL: [Self; 6] = [
        Self::Concurrency,
        Self::Failure,
        Self::Malformed,
        Self::Authz,
        Self::Replay,
        Self::Boundary,
    ];

    /// Stable kebab slug (also the mistakes-row + card-key fragment).
    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::Concurrency => "concurrency",
            Self::Failure => "failure",
            Self::Malformed => "malformed",
            Self::Authz => "authz",
            Self::Replay => "replay",
            Self::Boundary => "boundary",
        }
    }

    /// Adapt a finding from the shared `kavach_patterns::loophole_lens` kernel
    /// into this serde-carrying CLI lens (the YAML artifact needs serde; the
    /// kernel deliberately does not). One source of truth for the variants —
    /// adding a lens to the kernel forces this match to be updated.
    pub(crate) const fn from_kernel(k: kavach_patterns::loophole_lens::Lens) -> Self {
        use kavach_patterns::loophole_lens::Lens as K;
        match k {
            K::Concurrency => Self::Concurrency,
            K::Failure => Self::Failure,
            K::Malformed => Self::Malformed,
            K::Authz => Self::Authz,
            K::Replay => Self::Replay,
            K::Boundary => Self::Boundary,
        }
    }
}

/// One iteration of the loophole loop — the YAML artifact written to /tmp. It
/// pins exactly which round, which lenses, and which scope the agent must hunt,
/// so the work is precisely targeted (not a vague markdown instruction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoopholeIteration {
    /// Sweep run id (shared across all iterations of one sweep).
    pub run_id: String,
    /// 1-based round number within the sweep (the loop-until-dry counter).
    pub round: u32,
    /// Project slug whose findings/cards this iteration belongs to.
    pub project: String,
    /// Lenses to run this round (defaults to all six).
    pub lenses: Vec<Lens>,
    /// Repo-relative scope roots to hunt within (empty ⇒ whole workspace).
    #[serde(default)]
    pub scope: Vec<String>,
}

impl LoopholeIteration {
    /// Build the iteration for `round` of `run_id` over the full lens set.
    pub(crate) fn new(run_id: impl Into<String>, round: u32, project: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            round,
            project: project.into(),
            lenses: Lens::ALL.to_vec(),
            scope: Vec::new(),
        }
    }

    /// The canonical `/tmp/kavach-loophole/<run>/iter-<round>.yaml` path this
    /// iteration is written to. A FIXED `/tmp` root (not `std::env::temp_dir()`,
    /// which on macOS is a per-process `/var/folders/...` path) so the location is
    /// predictable and stable across invocations — the loop precisely targets the
    /// same working dir every round. Per-run subdir avoids concurrent-sweep
    /// collision; per-round file makes each unit individually targetable.
    pub(crate) fn tmp_path(&self) -> PathBuf {
        PathBuf::from("/tmp")
            .join("kavach-loophole")
            .join(&self.run_id)
            .join(format!("iter-{}.yaml", self.round))
    }

    /// Serialize to YAML text.
    pub(crate) fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Write this iteration's YAML to its /tmp path, creating parent dirs.
    /// Returns the path written — the precise unit-of-work target for the round.
    pub(crate) fn emit_tmp(&self) -> std::io::Result<PathBuf> {
        let path = self.tmp_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = self
            .to_yaml()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        // Atomic write: a reused run_id could collide two rounds on the same
        // path — temp+rename so no reader sees a partial iter file.
        let tmp = path.with_extension("yaml.tmp");
        std::fs::write(&tmp, yaml)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_lenses_have_unique_slugs() {
        let mut slugs: Vec<&str> = Lens::ALL.iter().map(|l| l.slug()).collect();
        assert_eq!(slugs.len(), 6);
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), 6, "lens slugs must be unique");
    }

    #[test]
    fn iteration_yaml_round_trips() {
        let it = LoopholeIteration::new("run-7", 2, "kavach-rs");
        let yaml = it.to_yaml().unwrap();
        let back: LoopholeIteration = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(it, back);
        assert_eq!(back.lenses.len(), 6);
    }

    #[test]
    fn tmp_path_is_per_run_per_round_under_tmp() {
        let it = LoopholeIteration::new("run-9", 3, "kavach-rs");
        let p = it.tmp_path();
        // Canonical, predictable /tmp root (not the per-process macOS temp dir).
        assert!(
            p.starts_with("/tmp/kavach-loophole"),
            "fixed /tmp root: {p:?}"
        );
        assert!(p.to_string_lossy().contains("kavach-loophole/run-9"));
        assert!(p.to_string_lossy().ends_with("iter-3.yaml"));
    }

    #[test]
    fn emit_writes_targetable_yaml_file() {
        let it = LoopholeIteration::new("run-emit-test", 1, "kavach-rs");
        let path = it.emit_tmp().unwrap();
        let read = std::fs::read_to_string(&path).unwrap();
        assert!(read.contains("run_id: run-emit-test"));
        assert!(read.contains("round: 1"));
        std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).ok();
    }
}
