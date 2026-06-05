use crate::loaders::get_agent_mappings;

#[must_use]
pub fn get_valid_agents() -> Vec<String> {
    let agents = get_agent_mappings();
    if let Some(list) = agents.get("VALID:AGENTS")
        && !list.is_empty()
    {
        return list.clone();
    }
    default_valid_agents()
}

#[must_use]
pub fn get_engineers() -> Vec<String> {
    let agents = get_agent_mappings();
    if let Some(list) = agents.get("ENGINEERS:LIST")
        && !list.is_empty()
    {
        return list.clone();
    }
    default_engineers()
}

#[must_use]
pub fn is_valid_agent(agent: &str) -> bool {
    get_valid_agents().iter().any(|a| a == agent)
}

#[must_use]
pub fn is_engineer(agent: &str) -> bool {
    get_engineers().iter().any(|e| e == agent)
}

pub fn default_valid_agents() -> Vec<String> {
    [
        "nlu-intent-analyzer",
        "ceo",
        "research-director",
        "backend-engineer",
        "frontend-engineer",
        "database-engineer",
        "devops-engineer",
        "security-engineer",
        "qa-lead",
        "aegis-guardian",
        "code-reviewer",
        "Explore",
        "Plan",
        "general-purpose",
        "Bash",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

pub fn default_engineers() -> Vec<String> {
    [
        "backend-engineer",
        "frontend-engineer",
        "database-engineer",
        "devops-engineer",
        "security-engineer",
        "qa-lead",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_valid_agents() {
        let agents = default_valid_agents();
        assert!(agents.contains(&"ceo".to_owned()));
        assert!(agents.contains(&"Bash".to_owned()));
        assert!(agents.len() >= 10);
    }
    #[test]
    fn test_default_engineers() {
        let engineers = default_engineers();
        assert!(engineers.contains(&"backend-engineer".to_owned()));
        assert!(!engineers.contains(&"ceo".to_owned()));
    }
}
