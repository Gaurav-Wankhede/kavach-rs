// The structural harness pattern the compiler emits for a goal — the six
// dynamic-workflow patterns. Each LLM sub-agent runs in its own clean context
// window, so generation never biases verification. Absent from YAML, defaults
// to `LoopUntilDone` (the original behavior), so legacy loop.yaml is unchanged.
//
// SOURCE: anthropic.com/research/building-effective-agents · youtube.com/watch?v=l5rae4LMKBc.
use serde::{Deserialize, Serialize};

/// One of six dynamic-workflow harness patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "pattern", rename_all = "kebab-case")]
pub(crate) enum Harness {
    /// Pattern 1 — a classifier routes the objective to one of `routes` by kind.
    ClassifyAct {
        /// Named downstream lanes (e.g. "fixer", "answerer"). One is chosen.
        routes: Vec<String>,
    },
    /// Pattern 2 — split the work across `shards` isolated agents, then a
    /// synthesis barrier merges every shard into one artifact.
    FanOutSynthesize {
        /// How many independent shards to fan the work across.
        shards: u32,
    },
    /// Pattern 3 — a worker produces an artifact; `critics` independent critics
    /// adversarially grade it against the oracle. Cures self-referential bias.
    WorkerCritic {
        /// How many independent critics must vote; majority decides.
        critics: u32,
    },
    /// Pattern 4 — generate `candidates` alternatives, then filter/dedup down to
    /// the survivors that pass the oracle.
    GenerateFilter {
        /// How many candidate artifacts to generate before filtering.
        candidates: u32,
    },
    /// Pattern 5 — `competitors` agents each solve the problem a different way;
    /// a judge compares head-to-head until one champion remains.
    PairwiseTournament {
        /// How many competing solutions enter the bracket (>= 2).
        competitors: u32,
    },
    /// Pattern 6 — the original oracle-gated loop: work, verify, fan out
    /// diagnostics on failure, until the oracle passes or limits run out.
    LoopUntilDone,
}

impl Default for Harness {
    /// Preserve the pre-enhancement behavior for any YAML lacking a `harness`.
    fn default() -> Self {
        Self::LoopUntilDone
    }
}
