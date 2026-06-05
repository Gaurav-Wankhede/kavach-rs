use std::collections::HashMap;

use chrono::Local;

use crate::chain_state::ChainState;
use crate::helpers::{is_dangerous_command, is_problematic_edit, is_sensitive_path};
use crate::types::{AegisVerification, IntentAnalysis, VerificationResult};

pub(crate) fn run_gate(
    state: &mut ChainState,
    tool_name: &str,
    tool_input: &HashMap<String, serde_json::Value>,
) {
    let aegis = aegis_verify(state.intent.as_ref(), tool_name, tool_input);

    let mut result = VerificationResult {
        gate: "AEGIS".into(),
        status: "pass".into(),
        reason: format!(
            "security_score={:.2} threat={}",
            aegis.security_score, aegis.threat_level
        ),
        context: HashMap::from([
            ("threat_level".into(), aegis.threat_level.clone()),
            (
                "security_score".into(),
                format!("{:.2}", aegis.security_score),
            ),
        ]),
        timestamp: String::new(),
        next_action: String::new(),
    };

    if !aegis.passed {
        result.status = "block".into();
        result.reason = aegis.violations_found.first().cloned().unwrap_or_default();
        result.next_action = "Address security violations before proceeding".into();
    }

    if let Some(rec) = aegis.recommendations.first() {
        result.context.insert("recommendations".into(), rec.clone());
    }

    state.aegis = Some(aegis);
    state.add_result(result);
}

#[must_use]
#[expect(
    clippy::implicit_hasher,
    reason = "callers pass the std HashMap; a generic S: BuildHasher bound adds no value at this gate boundary"
)]
pub fn aegis_verify(
    _intent: Option<&IntentAnalysis>,
    tool_name: &str,
    tool_input: &HashMap<String, serde_json::Value>,
) -> AegisVerification {
    let mut verification = AegisVerification {
        passed: true,
        security_score: 1.0,
        threat_level: "none".into(),
        violations_found: Vec::new(),
        recommendations: Vec::new(),
        memory_provenance: format!("chain_verification:{}", Local::now().to_rfc3339()),
    };

    if tool_name == "Bash"
        && let Some(cmd) = tool_input.get("command").and_then(|item| item.as_str())
        && is_dangerous_command(cmd)
    {
        verification.passed = false;
        verification.threat_level = "high".into();
        verification.security_score = 0.0;
        verification
            .violations_found
            .push("Dangerous command detected".into());
    }

    if matches!(tool_name, "Read" | "Write" | "Edit")
        && let Some(path) = tool_input.get("file_path").and_then(|item| item.as_str())
        && is_sensitive_path(path)
    {
        verification.passed = false;
        verification.threat_level = "high".into();
        verification.security_score = 0.0;
        verification
            .violations_found
            .push(format!("Sensitive file access: {path}"));
    }

    if tool_name == "Edit" {
        let old = tool_input
            .get("old_string")
            .and_then(|item| item.as_str())
            .unwrap_or("");
        let new = tool_input
            .get("new_string")
            .and_then(|item| item.as_str())
            .unwrap_or("");
        if is_problematic_edit(old, new) {
            verification.passed = false;
            verification.threat_level = "medium".into();
            verification.security_score = 0.3;
            verification
                .violations_found
                .push("Suspicious code removal - verify intent".into());
        }
    }

    verification
}

#[cfg(test)]
#[path = "aegis_tests.rs"]
mod tests;
