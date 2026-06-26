use crate::config::load;

#[must_use]
pub fn is_valid_agent(agent: &str) -> bool {
    let g = load();
    if let Some(c) = g.as_ref() {
        for a in c.valid_agents.values() {
            if a.iter().any(|x| x == agent) {
                return true;
            }
        }
    }
    drop(g);
    ["Explore", "Plan", "Bash"].contains(&agent)
}

#[must_use]
pub fn classify_intent(prompt: &str) -> String {
    let g = load();
    let pl = prompt.to_lowercase();
    if let Some(c) = g.as_ref() {
        for (i, w) in &c.intent_words {
            if w.iter().any(|x| pl.contains(x)) {
                return i.clone();
            }
        }
    }
    drop(g);
    "general".into()
}
