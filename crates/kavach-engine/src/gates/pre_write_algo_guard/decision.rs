//! Prior-decision lookup via the kavach-rpc daemon (auto-inject path).

/// Query the daemon for a prior algorithm decision matching this project +
/// trigger keyword. `None` if the daemon is down or no decision matches — the
/// gate then degrades to the Block path.
pub(super) fn load_prior_decision(project_slug: &str, trigger_keyword: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({"project": project_slug, "limit": 5});
    let recent: Vec<kavach_surreal::AlgoDecision> =
        kavach_rpc::client::call("algo.list_recent", Some(params)).ok()?;
    if recent.is_empty() {
        return None;
    }
    let matched = recent.iter().find(|d| {
        d.chosen.contains(trigger_keyword)
            || d.problem_class.contains(trigger_keyword)
            || trigger_keyword.contains(&d.problem_class)
    });
    let decision = match matched {
        Some(d) => d,
        None => recent.first()?,
    };
    Some(format!(
        "[ALGO_AUTO_INJECT]\nstatus: prior_decision_found\n\
         problem_class: {}\nchosen: {}\ntime: {}\nspace: {}\n\
         searched: {}-{:02}\nfile: {}\n\
         advisory: Prior decision injected — confirm it still applies or re-run /arch",
        decision.problem_class,
        decision.chosen,
        decision.time_complexity,
        decision.space_complexity,
        decision.search_year,
        decision.search_month,
        decision.file_path,
    ))
}
