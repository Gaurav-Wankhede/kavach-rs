//! The JSON role-query the LLM passes, and the candidate the walker yields.

use super::site::Kind;
use serde::Deserialize;

/// A role descriptor — a UNION of signals; every field is optional.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(super) struct RoleQuery {
    pub role: String,
    pub value_regex: Option<String>,
    pub consumed_by: Vec<String>,
    pub env_key_hints: Vec<String>,
    pub name_aliases: Vec<String>,
}

impl RoleQuery {
    pub(super) fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid role-query JSON: {e}"))
    }
}

/// One declaration site enriched with its captured value + secret flag.
#[derive(Debug, Clone)]
pub(super) struct Candidate {
    pub name: String,
    pub kind: Kind,
    pub file: String,
    pub line: usize,
    pub value: Option<String>,
    pub is_secret: bool,
}

#[cfg(test)]
#[path = "role_query_test.rs"]
mod role_query_test;
