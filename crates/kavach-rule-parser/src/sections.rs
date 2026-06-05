//! Section extraction from TOON skill files

use kavach_toon::Document;

#[derive(Debug, Default)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed and matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct Sections {
    pub research_gate: Option<String>,
    pub error_handling: Option<String>,
    pub pending_tasks: Option<String>,
    pub async_rules: Option<String>,
    pub do_dont: Option<String>,
}

impl Sections {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.research_gate.is_none()
            && self.error_handling.is_none()
            && self.pending_tasks.is_none()
            && self.async_rules.is_none()
            && self.do_dont.is_none()
    }
}

#[must_use]
pub fn extract_sections(doc: &Document) -> Sections {
    let mut sections = Sections::new();

    for (name, block) in &doc.blocks {
        match name.to_uppercase().as_str() {
            "RESEARCH_GATE" => {
                sections.research_gate = Some(block.to_string());
            }
            "ERROR_HANDLING" => {
                sections.error_handling = Some(block.to_string());
            }
            "PENDING_TASKS" => {
                sections.pending_tasks = Some(block.to_string());
            }
            "ASYNC" => {
                sections.async_rules = Some(block.to_string());
            }
            "RULES" => {
                sections.do_dont = Some(block.to_string());
            }
            _ => {}
        }
    }

    sections
}
