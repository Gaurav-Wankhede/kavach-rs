//! OpenAI-compatible wire contract for the orchestrator — "system-as-a-model".
//!
//! Pure (de)serialization + request↔[`VendorRequest`] mapping so any OpenAI
//! chat-completions client can drive the kavach vendor pool. The eventual Axum
//! handler is a thin shell over [`request_to_vendor`] → dispatch →
//! [`vendor_to_response`]; this module carries no I/O.
//!
//! SOURCE: decision.fugu-orchestration-layer · developers.openai.com/api/reference
use serde::{Deserialize, Serialize};

use super::{role_for_title, AgentRole, VendorOutput, VendorRequest};

/// One chat message (`{role, content}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatMessage {
    /// `system` | `user` | `assistant`.
    pub role: String,
    /// Message text.
    pub content: String,
}

/// An OpenAI `POST /v1/chat/completions` request (subset kavach consumes).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletionRequest {
    /// Requested model id (echoed back; routing is by role, not model).
    pub model: String,
    /// Conversation so far.
    pub messages: Vec<ChatMessage>,
}

/// One choice in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatChoice {
    /// Position in the choices array.
    pub index: u32,
    /// The assistant message.
    pub message: ChatMessage,
    /// Why generation stopped (`stop`).
    pub finish_reason: String,
}

/// An OpenAI chat-completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletionResponse {
    /// Constant `chat.completion`.
    pub object: String,
    /// Echoed model id.
    pub model: String,
    /// Exactly one choice (the orchestrated result).
    pub choices: Vec<ChatChoice>,
}

/// Map a chat request to a [`VendorRequest`]: the last `user` message is the
/// prompt; its text classifies the TRINITY role. No user message → empty Worker.
#[must_use]
pub fn request_to_vendor(req: &ChatCompletionRequest) -> VendorRequest {
    let prompt = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map_or_else(String::new, |m| m.content.clone());
    let role = if prompt.is_empty() {
        AgentRole::Worker
    } else {
        role_for_title(&prompt)
    };
    VendorRequest {
        role,
        prompt,
        project: req.model.clone(),
        max_turns: 8,
    }
}

/// Wrap a [`VendorOutput`] as a single-choice chat completion.
#[must_use]
pub fn vendor_to_response(out: &VendorOutput, model: &str) -> ChatCompletionResponse {
    ChatCompletionResponse {
        object: "chat.completion".to_owned(),
        model: model.to_owned(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_owned(),
                content: out.stdout.clone(),
            },
            finish_reason: "stop".to_owned(),
        }],
    }
}

#[cfg(test)]
#[path = "orchestrate_test.rs"]
mod orchestrate_test;
