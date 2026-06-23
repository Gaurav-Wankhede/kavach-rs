//! TDD: OpenAI-compatible wire contract + pure request↔VendorRequest mapping.
//! "system-as-a-model" — any OpenAI client consumes the orchestrator.
//! SOURCE: decision.fugu-orchestration-layer · developers.openai.com/api/reference
use super::*;

fn chat_req() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "kavach-fugu".into(),
        messages: vec![
            ChatMessage { role: "system".into(), content: "be terse".into() },
            ChatMessage { role: "user".into(), content: "[PLAN] do X".into() },
        ],
    }
}

#[test]
fn request_maps_last_user_message_to_prompt() {
    let v = request_to_vendor(&chat_req());
    assert_eq!(v.prompt, "[PLAN] do X");
}

#[test]
fn role_is_inferred_from_the_prompt() {
    // "[PLAN] ..." classifies Thinker via the role classifier.
    assert_eq!(request_to_vendor(&chat_req()).role, AgentRole::Thinker);
}

#[test]
fn request_with_no_user_message_is_worker_empty() {
    let req = ChatCompletionRequest {
        model: "m".into(),
        messages: vec![ChatMessage { role: "system".into(), content: "x".into() }],
    };
    let v = request_to_vendor(&req);
    assert_eq!(v.prompt, "");
    assert_eq!(v.role, AgentRole::Worker);
}

#[test]
fn vendor_output_maps_to_chat_completion() {
    let out = VendorOutput { vendor: "cc".into(), stdout: "done".into(), exit_code: 0 };
    let resp = vendor_to_response(&out, "kavach-fugu");
    assert_eq!(resp.object, "chat.completion");
    assert_eq!(resp.model, "kavach-fugu");
    assert_eq!(resp.choices.len(), 1);
    assert_eq!(resp.choices[0].message.role, "assistant");
    assert_eq!(resp.choices[0].message.content, "done");
    assert_eq!(resp.choices[0].finish_reason, "stop");
}

#[test]
fn response_round_trips_through_json() {
    let out = VendorOutput { vendor: "codex".into(), stdout: "ok".into(), exit_code: 0 };
    let resp = vendor_to_response(&out, "m");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("\"object\":\"chat.completion\""));
    assert!(json.contains("\"role\":\"assistant\""));
}
