use serde::{Deserialize, Serialize};

/// A single message in the conversation.
#[derive(Debug, Clone, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct Message {
    pub role: &'static str,
    pub content: String,
}

/// The advisor server-side tool declaration.
#[derive(Debug, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct AdvisorTool {
    #[serde(rename = "type")]
    pub tool_type: &'static str,
    pub name: &'static str,
    pub model: &'static str,
    pub max_uses: u8,
}

impl Default for AdvisorTool {
    fn default() -> Self {
        Self {
            tool_type: "advisor_20260301",
            name: "advisor",
            model: "claude-opus-4-6",
            max_uses: 3,
        }
    }
}

/// Full request body sent to `/v1/messages`.
#[derive(Debug, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct MessagesRequest<'a> {
    pub model: &'a str,
    pub max_tokens: u32,
    pub tools: Vec<AdvisorTool>,
    pub messages: Vec<Message>,
}

/// One content block in the API response.
#[derive(Debug, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

/// Top-level API response.
#[derive(Debug, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct MessagesResponse {
    pub content: Vec<ContentBlock>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_set_correct_defaults_for_advisor_tool() {
        let tool = AdvisorTool::default();
        assert_eq!(tool.tool_type, "advisor_20260301");
        assert_eq!(tool.model, "claude-opus-4-6");
        assert_eq!(tool.max_uses, 3);
    }

    #[test]
    fn should_deserialize_text_content_block() {
        let json = r#"{"type":"text","text":"hello"}"#;
        let block: ContentBlock = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(block.block_type, "text");
        assert_eq!(block.text.as_deref(), Some("hello"));
    }

    #[test]
    fn should_deserialize_non_text_block_without_text() {
        let json = r#"{"type":"tool_use"}"#;
        let block: ContentBlock = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(block.block_type, "tool_use");
        assert!(block.text.is_none());
    }
}
