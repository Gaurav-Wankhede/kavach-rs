use kavach_patterns::eval_replay::TrajectoryEvent;
use kavach_patterns::reward;
pub(super) fn path_is_gate_or_dispatch(path: &str) -> bool {
    path.contains("/gates/")
        || path.contains("/stop_dispatch/")
        || path == "crates/kavach-patterns/src/reward.rs"
}
pub(super) fn touched_gate_or_dispatch(card_key: &str) -> bool {
    if card_key.is_empty() {
        return false;
    }
    let Ok(output) = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD~1", "HEAD"])
        .output()
    else {
        return false;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().any(path_is_gate_or_dispatch)
}
pub(super) fn eval_trajectory_score(events: &[TrajectoryEvent]) -> i64 {
    let rubric = reward::presets::rust_cargo();
    reward::score_trajectory_with(events, &rubric)
}
pub(super) fn eval_advisory(_project_slug: &str, card_key: &str) -> Option<String> {
    if !touched_gate_or_dispatch(card_key) {
        return None;
    }
    let path = kavach_patterns::eval_replay::default_trajectory_path(card_key).ok()?;
    let events = kavach_patterns::eval_replay::read_jsonl(&path).ok()?;
    let score = eval_trajectory_score(&events);
    if score >= 0 {
        return None;
    }
    Some(format!(
        "[TRAJECTORY_EVAL_P1] card {card_key} touched gate/dispatch; trajectory score {score} < 0 — review the turn's tool sequence (deferral/missing-verify). Promotion NOT blocked (advisory)."
    ))
}
#[cfg(test)]
#[path = "trajectory_eval_test.rs"]
mod tests;
