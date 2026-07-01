//! Advisory Opus/Sonnet router from turn complexity+risk+card-category vs live model; hooks can't set model. SOURCE: https://code.claude.com/docs/en/hooks
use kavach_types::HookInput;
#[cfg(test)]
#[path = "model_route_tests.rs"]
mod tests;
const TOP_TIER_PREFIX: &str = "claude-opus";
const FORCE_OPUS_CATEGORIES: [&str; 3] = ["security", "rca", "architecture"];
/// Append a `[MODEL_ROUTE]` advisory when the live model tier mismatches the turn's demand.
pub(crate) fn append_model_route(
    context: &mut String,
    input: &HookInput,
    complexity: &str,
    risk: &str,
    project: &str,
    prompt: &str,
    session_id: &str,
) {
    let model = input.model.as_str();
    let nlu_escalates = complexity == "complex" || risk == "high";
    let on_top = model.starts_with(TOP_TIER_PREFIX);
    // Category can only ADD escalation — pay the RPC only when it could change the outcome.
    let forced = if !nlu_escalates && !on_top {
        top_card_category(project, prompt).filter(|c| FORCE_OPUS_CATEGORIES.contains(&c.as_str()))
    } else {
        None
    };
    let Some(line) = recommend(model, complexity, risk, forced.as_deref()) else {
        return;
    };
    let alias = if line.contains("recommend:opus") {
        "opus"
    } else {
        "sonnet"
    };
    context.push_str(&line);
    crate::gates::event_log::log_model_route(session_id, model, alias, complexity, project);
    maybe_autoswitch(alias);
}
fn recommend(
    model: &str,
    complexity: &str,
    risk: &str,
    forced_category: Option<&str>,
) -> Option<String> {
    if !model.starts_with("claude-") {
        return None;
    }
    let on_top = model.starts_with(TOP_TIER_PREFIX);
    let wants_top = complexity == "complex" || risk == "high" || forced_category.is_some();
    match (wants_top, on_top) {
        (true, false) => {
            let reason = forced_category.map_or_else(
                || format!("complexity={complexity}/risk={risk}"),
                |c| format!("category={c} (forces opus)"),
            );
            Some(format!(
                "\n[MODEL_ROUTE] current:{model} · recommend:opus · reason:{reason} — escalate for gated reasoning (RCA/loophole/three-witness). Switch: /model opus\n"
            ))
        }
        (false, true) => Some(format!(
            "\n[MODEL_ROUTE] current:{model} · recommend:sonnet · reason:complexity={complexity}/risk={risk} — mechanical turn; downgrade to save cost. Switch: /model sonnet\n"
        )),
        _ => None,
    }
}
fn top_card_category(project: &str, prompt: &str) -> Option<String> {
    if project.is_empty() {
        return None;
    }
    let params = serde_json::json!({ "project": project, "prompt": prompt, "limit": 1 });
    let val =
        kavach_rpc::client::call::<_, serde_json::Value>("db.kanban_ranked", Some(params)).ok()?;
    val.get("cards")?
        .as_array()?
        .first()?
        .get("category")?
        .as_str()
        .map(str::to_owned)
}
fn maybe_autoswitch(alias: &str) {
    if !kavach_config::load_gates_config().model.autoswitch {
        return;
    }
    let Ok(home) = std::env::var("HOME") else {
        return;
    };
    let path = std::path::Path::new(&home).join(".claude").join("settings.json");
    let Ok(data) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&data) else {
        return;
    };
    let Some(obj) = json.as_object_mut() else {
        return;
    };
    if obj.get("model").and_then(serde_json::Value::as_str) == Some(alias) {
        return;
    }
    obj.insert("model".to_owned(), serde_json::Value::String(alias.to_owned()));
    if let Ok(out) = serde_json::to_string_pretty(&json) {
        drop(std::fs::write(&path, out));
    }
}
