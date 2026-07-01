//! Advisory Opus/Sonnet router from turn complexity+risk vs live model; hooks can't set model. SOURCE: https://code.claude.com/docs/en/hooks
use kavach_types::HookInput;
#[cfg(test)]
#[path = "model_route_tests.rs"]
mod tests;
const TOP_TIER_PREFIX: &str = "claude-opus";
/// Append a `[MODEL_ROUTE]` advisory when the live model tier mismatches the turn's demand.
pub(crate) fn append_model_route(context: &mut String, input: &HookInput, complexity: &str, risk: &str) {
    if let Some(line) = recommend(&input.model, complexity, risk) {
        context.push_str(&line);
    }
}
fn recommend(model: &str, complexity: &str, risk: &str) -> Option<String> {
    if !model.starts_with("claude-") {
        return None;
    }
    let wants_top = complexity == "complex" || risk == "high";
    let on_top = model.starts_with(TOP_TIER_PREFIX);
    match (wants_top, on_top) {
        (true, false) => Some(format!(
            "\n[MODEL_ROUTE] current:{model} · recommend:opus · reason:complexity={complexity}/risk={risk} — escalate for gated reasoning (RCA/loophole/three-witness). Switch: /model opus\n"
        )),
        (false, true) => Some(format!(
            "\n[MODEL_ROUTE] current:{model} · recommend:sonnet · reason:complexity={complexity}/risk={risk} — mechanical turn; downgrade to save cost. Switch: /model sonnet\n"
        )),
        _ => None,
    }
}
