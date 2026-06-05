// SOURCE: https://docs.rs/jsonrpsee (sync UDS adapter via kavach-rpc::client)
use serde::{Deserialize, Serialize};

use crate::rpc_client::{Error as RpcError, rpc};

#[derive(Debug, Serialize)]
struct SearchParams<'a> {
    query: &'a str,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ListParams {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct AddParams<'a> {
    name: &'a str,
    display: &'a str,
    desc: &'a str,
    tags: Option<Vec<String>>,
    sources: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct ConceptDto {
    pub name: String,
    pub entity_type: String,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum LoadState {
    Ok(Vec<ConceptDto>),
    DaemonOffline,
}

pub fn list_concepts(limit: usize) -> LoadState {
    match rpc::<ListParams, Vec<ConceptDto>>("concept.list", ListParams { limit: Some(limit) }) {
        Ok(v) => LoadState::Ok(v),
        Err(RpcError::DaemonOffline(_)) => LoadState::DaemonOffline,
        Err(e) => {
            tracing::error!(error = %e, "concept.list failed");
            LoadState::Ok(Vec::new())
        }
    }
}

pub fn search_concepts(q: &str, limit: usize) -> LoadState {
    let res = rpc::<SearchParams<'_>, Vec<ConceptDto>>(
        "concept.search",
        SearchParams {
            query: q,
            limit: Some(limit),
        },
    );
    match res {
        Ok(v) => LoadState::Ok(v),
        Err(RpcError::DaemonOffline(_)) => LoadState::DaemonOffline,
        Err(e) => {
            tracing::error!(error = %e, "concept.search failed");
            LoadState::Ok(Vec::new())
        }
    }
}

pub fn add_concept(name: &str, display: &str, desc: &str, source_url: &str) -> Result<(), String> {
    let sources = if source_url.is_empty() {
        None
    } else {
        Some(vec![source_url.to_owned()])
    };
    let res = rpc::<AddParams<'_>, serde_json::Value>(
        "concept.add",
        AddParams {
            name,
            display,
            desc,
            tags: None,
            sources,
        },
    );
    match res {
        Ok(_) => Ok(()),
        Err(RpcError::DaemonOffline(_)) => Err("kavach-rpc daemon offline".to_owned()),
        Err(RpcError::Rpc { code, message }) => Err(format!("rpc {code}: {message}")),
        Err(e) => Err(e.to_string()),
    }
}

// Removed per FIX_REQUIRED P3 TYPE_LOOSE (allow(dead_code) suppression).
// rpc_no_params is available via crate::rpc_client when needed.
