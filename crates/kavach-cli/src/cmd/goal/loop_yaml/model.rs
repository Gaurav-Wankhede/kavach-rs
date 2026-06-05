// GoalLoopYaml struct + serde/disk round-trip. The full declarative
// source-of-truth written to `.kavach/goals/<id>/loop.yaml`.
//
// SOURCE: decision.goal-oracle-workflow · decision.goal-harness-6-patterns.
use super::{Harness, LoopLimits, Oracle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The full goal-loop declaration written to `.kavach/goals/<id>/loop.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GoalLoopYaml {
    /// Stable slug identifying this goal (also the on-disk directory name).
    pub goal_id: String,
    /// Human statement of the target outcome.
    pub intent: String,
    /// The proof signal that gates completion.
    pub oracle: Oracle,
    /// The structural harness pattern the compiler emits. Defaults to
    /// `LoopUntilDone` so pre-enhancement loop.yaml files compile unchanged.
    #[serde(default)]
    pub harness: Harness,
    /// Loop-control limits.
    #[serde(default)]
    pub loop_limits: LoopLimits,
}

impl GoalLoopYaml {
    /// Build a `test-exit-code` goal — the MVP shape.
    pub(crate) fn test_exit_code(
        goal_id: impl Into<String>,
        intent: impl Into<String>,
        check: impl Into<String>,
    ) -> Self {
        Self {
            goal_id: goal_id.into(),
            intent: intent.into(),
            oracle: Oracle::TestExitCode {
                check: check.into(),
                expect_contains: None,
            },
            harness: Harness::default(),
            loop_limits: LoopLimits::default(),
        }
    }

    /// Serialize to YAML text.
    pub(crate) fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Parse from YAML text. The compiler reads loop.yaml back from disk.
    pub(crate) fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    /// Read and parse a loop.yaml from disk at `path`.
    pub(crate) fn read(path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::from_yaml(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Repo-relative path where this goal's loop file lives.
    pub(crate) fn loop_path(&self) -> PathBuf {
        Path::new(".kavach")
            .join("goals")
            .join(&self.goal_id)
            .join("loop.yaml")
    }

    /// Write the loop file under `root`, creating parent dirs. Returns the path
    /// written. The on-disk YAML is the source of truth the compiler reads.
    pub(crate) fn emit(&self, root: &Path) -> std::io::Result<PathBuf> {
        let rel = self.loop_path();
        let abs = root.join(&rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = self
            .to_yaml()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&abs, yaml)?;
        Ok(rel)
    }
}
