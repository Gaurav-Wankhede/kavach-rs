//! Advisory orchestrator: detect → load spec → emit `[GNAP_SPEC_REF]` block.
use super::detect::is_auth_related;
use super::extract::extract_relevant_sections;
use kavach_config::gnap_spec_path;
use std::fs;

const QUICK_REF: &str = "## Quick Reference\n\n\
     - Grant endpoint: POST /gnap/grants\n\
     - Key proofing: httpsig (RFC 9421) with Ed25519\n\
     - Token binding: All tokens key-bound by default\n\
     - Introspection: POST /gnap/introspect (RS validates)\n\
     - BANNED: Bearer headers, client_id/secret, OAuth flows\n";

/// Returns advisory context with GNAP spec excerpts if auth patterns detected.
pub(crate) fn advisory(file_path: &str, content: &str) -> Option<String> {
    if !is_auth_related(file_path, content) {
        return None;
    }
    let spec_path = gnap_spec_path();
    if !spec_path.exists() {
        return Some(
            "[GNAP_SPEC_REF]\n\
             status: spec_not_found\n\
             action: Read docs/superpowers/specs/2026-04-25-gnap-rfc9635-rfc9767-implementation.md\n\
             stack: GNAP (RFC 9635) + httpsig (RFC 9421) + PASETO v4 + Ed25519\n"
                .to_owned(),
        );
    }
    let Ok(spec_content) = fs::read_to_string(&spec_path) else {
        return Some(
            "[GNAP_SPEC_REF]\n\
             status: spec_read_error\n\
             action: Check docs/superpowers/specs/ directory\n"
                .to_owned(),
        );
    };

    let sections = extract_relevant_sections(&spec_content, content);
    let mut block = String::from("[GNAP_SPEC_REF]\n");
    block.push_str(
        "source: docs/superpowers/specs/2026-04-25-gnap-rfc9635-rfc9767-implementation.md\n",
    );
    block.push_str("rfcs: RFC 9635 (GNAP Core) + RFC 9767 (Resource Servers)\n\n");
    if sections.is_empty() {
        block.push_str(QUICK_REF);
    } else {
        block.push_str(&sections);
    }
    Some(block)
}
