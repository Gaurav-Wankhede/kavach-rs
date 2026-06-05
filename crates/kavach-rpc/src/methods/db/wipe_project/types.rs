// ALGO: String concatenation
//! Wipe project RPC types and helpers.

use serde::{Deserialize, Serialize};

const WIPE_PREFIX: &str = "wipe ";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct WipeProjectParams {
    pub project: String,
    #[serde(default)]
    pub dry_run: Option<bool>,
    #[serde(default)]
    pub confirm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct WipeProjectResult {
    pub success: bool,
    pub report: Option<WipeReportDto>,
    pub dry_run: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct WipeReportDto {
    pub project_slug: String,
    pub tables: Vec<(String, usize)>,
    pub project_deleted: bool,
}

/// Build user confirmation phrase for wiping entire project.
#[must_use]
pub fn wipe_confirm_phrase(project: &str) -> String {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "capacity estimation, operands bounded"
    )]
    let mut phrase = String::with_capacity(project.len() + WIPE_PREFIX.len());
    phrase.push_str(WIPE_PREFIX);
    phrase.push_str(project);
    phrase
}

/// Build error message for confirmation failure.
#[must_use]
pub(super) fn wipe_error_msg(expected: &str) -> String {
    let mut msg = String::with_capacity(256);
    msg.push_str("destructive project wipe requires confirmation — resend with confirm = \"");
    msg.push_str(expected);
    msg.push('"');
    msg
}
