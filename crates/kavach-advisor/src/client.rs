use crate::error::AdvisorError;
use crate::types::{AdvisorTool, ContentBlock, Message, MessagesRequest, MessagesResponse};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_BETA: &str = "advisor-tool-2026-03-01";
/// Haiku 4.5 — executor model (fast, cheap).
const EXECUTOR_MODEL: &str = "claude-haiku-4-5-20251001";

/// Send `prompt` to Haiku with Opus as advisor.
///
/// Reads `ANTHROPIC_API_KEY` from the environment.
/// Returns the first `text` block from the response.
///
/// # Errors
///
/// Returns an error if the API key is missing, invalid UTF-8, the request fails, or no text block is present in the response.
pub fn ask(prompt: &str, max_uses: u8) -> Result<String, AdvisorError> {
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(v) => v,
        Err(std::env::VarError::NotPresent) => return Err(AdvisorError::MissingApiKey),
        Err(std::env::VarError::NotUnicode(os)) => return Err(AdvisorError::ApiKeyNotUnicode(os)),
    };

    let body = MessagesRequest {
        model: EXECUTOR_MODEL,
        max_tokens: 1024,
        tools: vec![AdvisorTool {
            max_uses,
            ..AdvisorTool::default()
        }],
        messages: vec![Message {
            role: "user",
            content: prompt.to_owned(),
        }],
    };

    let response: MessagesResponse = ureq::post(API_URL)
        .header("x-api-key", &api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("anthropic-beta", ANTHROPIC_BETA)
        .header("content-type", "application/json")
        .send_json(&body)?
        .body_mut()
        .read_json()?;

    extract_text(response.content)
}

fn extract_text(blocks: Vec<ContentBlock>) -> Result<String, AdvisorError> {
    for block in blocks {
        if block.block_type == "text"
            && let Some(text) = block.text
        {
            return Ok(text);
        }
    }
    Err(AdvisorError::NoTextBlock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContentBlock;

    #[test]
    fn should_return_text_when_first_block_is_text() {
        let blocks = vec![ContentBlock {
            block_type: "text".to_owned(),
            text: Some("hello".to_owned()),
        }];
        let result = extract_text(blocks).expect("should extract text");
        assert_eq!(result, "hello");
    }

    #[test]
    fn should_skip_non_text_blocks_and_return_text() {
        let blocks = vec![
            ContentBlock {
                block_type: "tool_use".to_owned(),
                text: None,
            },
            ContentBlock {
                block_type: "text".to_owned(),
                text: Some("found".to_owned()),
            },
        ];
        let result = extract_text(blocks).expect("should find text block");
        assert_eq!(result, "found");
    }

    #[test]
    fn should_error_when_no_text_block_present() {
        let blocks = vec![ContentBlock {
            block_type: "tool_use".to_owned(),
            text: None,
        }];
        let err = extract_text(blocks);
        assert!(matches!(err, Err(AdvisorError::NoTextBlock)));
    }

    #[test]
    fn should_error_when_blocks_empty() {
        let err = extract_text(vec![]);
        assert!(matches!(err, Err(AdvisorError::NoTextBlock)));
    }
}
