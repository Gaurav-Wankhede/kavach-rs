//! Skill validation rules

#[derive(Debug, thiserror::Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "error type matched cross-crate; non_exhaustive => E0639"
)]
pub enum ValidationError {
    #[error("Missing required section: {0}")]
    MissingSection(String),
    #[error("Invalid skill name: must be snake_case")]
    InvalidSkillName,
    #[error("Triggers must be non-empty list")]
    EmptyTriggers,
    #[error("Skill protocol must be SP/3.0")]
    InvalidProtocol,
}

/// Validates a skill file's frontmatter and required sections.
///
/// # Errors
///
/// Returns `ValidationError` if required sections are missing, skill name is invalid, or protocol is not SP/3.0.
pub fn validate_skill(
    frontmatter_name: &str,
    sections: &super::sections::Sections,
) -> Result<(), ValidationError> {
    if sections.research_gate.is_none() {
        return Err(ValidationError::MissingSection("RESEARCH_GATE".into()));
    }

    if sections.error_handling.is_none() {
        return Err(ValidationError::MissingSection("ERROR_HANDLING".into()));
    }

    if sections.pending_tasks.is_none() {
        return Err(ValidationError::MissingSection("PENDING_TASKS".into()));
    }

    if !frontmatter_name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c == '_' || c == '-')
    {
        return Err(ValidationError::InvalidSkillName);
    }

    let protocol = frontmatter_name.split(':').next_back().unwrap_or("SP/1.0");
    if protocol != "SP/3.0" {
        return Err(ValidationError::InvalidProtocol);
    }

    Ok(())
}
