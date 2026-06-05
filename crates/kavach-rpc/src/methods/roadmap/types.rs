use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct EntryStatusParams {
    pub project: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct EntryStatusResult {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct NextOpenTaskParams {
    pub project: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct NextTaskResult {
    pub key: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ListTitlesParams {
    pub project: String,
    #[serde(default)]
    pub category: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct TitleRow {
    pub category: String,
    pub key: String,
    pub title: String,
    #[serde(rename = "entry_status")]
    pub entry_status: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ClaimCardParams {
    pub project: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ClaimCardResult {
    pub key: String,
    pub status: String,
    pub claimed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct VerifyCardResult {
    pub key: String,
    pub status: String,
    pub verified: bool,
}
