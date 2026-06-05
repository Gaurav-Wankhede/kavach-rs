// ALGO: String concatenation
//! Delete RPC types and helpers.

use serde::{Deserialize, Serialize};

const CONFIRM_PREFIX: &str = "delete ";
const SEPARATOR: &str = "/";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct DeleteParams {
    pub project: String,
    pub category: String,
    pub key: Option<String>,
    #[serde(default)]
    pub all: Option<bool>,
    #[serde(default)]
    pub dry_run: Option<bool>,
    #[serde(default)]
    pub confirm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct DeleteResult {
    pub success: bool,
    pub deleted_count: usize,
    pub dry_run: bool,
    pub error: Option<String>,
}

/// Build user confirmation phrase for destructive delete.
#[must_use]
pub fn delete_confirm_phrase(project: &str, category: &str, key: Option<&str>) -> String {
    let mut phrase = String::with_capacity(256);
    phrase.push_str(CONFIRM_PREFIX);
    phrase.push_str(project);
    phrase.push_str(SEPARATOR);
    phrase.push_str(category);
    if let Some(k) = key {
        phrase.push_str(SEPARATOR);
        phrase.push_str(k);
    }
    phrase
}

/// Build error message for confirmation failure.
#[must_use]
pub(super) fn confirmation_error_msg(expected: &str) -> String {
    let mut msg = String::with_capacity(256);
    msg.push_str("destructive delete requires confirmation — resend with confirm = \"");
    msg.push_str(expected);
    msg.push('"');
    msg
}
