// Goal -> Claude Code Workflow compiler (hub).
//
// Dispatches a GoalLoopYaml to one of six dynamic-workflow harness emitters by
// its `harness` pattern, writes the resulting `workflow.js`, and backs the
// `kavach goal compile` CLI verb. Each emitter lives in its own leaf module so
// every pattern is testable in isolation.
//
// The six patterns map onto Anthropic's canonical agent workflows (routing,
// parallelization, orchestrator-workers, evaluator-optimizer) and cure the
// static-harness failure modes: laziness (loop-until-done), self-referential
// bias (worker-critic, pairwise-tournament), and goal drift (isolated contexts).
//
// SOURCE: anthropic.com/research/building-effective-agents · decision.goal-harness-6-patterns.
mod classify_act;
mod escape;
mod fan_out_synthesize;
mod generate_filter;
mod loop_until_done;
mod model_tier;
mod pairwise_tournament;
mod worker_critic;
#[cfg(test)]
#[path = "compile_test.rs"]
mod tests;
use super::loop_yaml::{GoalLoopYaml, Harness};
use std::path::{Path, PathBuf};
/// Render a goal as a Claude Code Workflow script, dispatching on its harness
/// pattern. Absent harness (legacy YAML) compiles as the Pattern-6 loop.
pub(crate) fn to_workflow_js(goal_yaml: &GoalLoopYaml) -> String {
    match &goal_yaml.harness {
        Harness::ClassifyAct { routes } => classify_act::emit(goal_yaml, routes),
        Harness::FanOutSynthesize { shards } => fan_out_synthesize::emit(goal_yaml, *shards),
        Harness::WorkerCritic { critics } => worker_critic::emit(goal_yaml, *critics),
        Harness::GenerateFilter { candidates } => generate_filter::emit(goal_yaml, *candidates),
        Harness::PairwiseTournament { competitors } => {
            pairwise_tournament::emit(goal_yaml, *competitors)
        }
        Harness::LoopUntilDone => loop_until_done::emit(goal_yaml),
    }
}
/// Compile a goal's `loop.yaml` to a sibling `workflow.js` under `root`.
/// Returns the repo-relative path written.
pub(crate) fn compile_to_workflow(
    goal_yaml: &GoalLoopYaml,
    root: &Path,
) -> std::io::Result<PathBuf> {
    let rel = Path::new(".kavach")
        .join("goals")
        .join(&goal_yaml.goal_id)
        .join("workflow.js");
    let abs = root.join(&rel);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Atomic write: a concurrent compile (or a reader mid-write) must never
    // observe a half-written workflow.js. Write to a temp sibling then rename
    // (atomic on the same filesystem). SOURCE: decision.goal-reconcile-lost-update-fix.
    let tmp = abs.with_extension("js.tmp");
    std::fs::write(&tmp, to_workflow_js(goal_yaml))?;
    std::fs::rename(&tmp, &abs)?;
    Ok(rel)
}
/// `kavach goal compile` — read a goal's loop.yaml and emit its workflow.js.
pub(crate) fn run(goal_id: &str) -> i32 {
    let yaml_path = Path::new(".kavach")
        .join("goals")
        .join(goal_id)
        .join("loop.yaml");
    let goal = match GoalLoopYaml::read(&yaml_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("kavach goal compile: read {}: {e}", yaml_path.display());
            return 1;
        }
    };
    match compile_to_workflow(&goal, Path::new(".")) {
        Ok(path) => {
            println!("[WORKFLOW_COMPILED] {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("kavach goal compile: write workflow.js: {e}");
            1
        }
    }
}
