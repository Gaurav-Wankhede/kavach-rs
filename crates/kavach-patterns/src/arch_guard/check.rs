//! Architecture guard check and advisory logic.

use super::detect::{count_arch_fields, detect, has_arch_comment};
use super::types::ArchGuardOutcome;

const REQUIRED_FIELDS: usize = 9;

/// Advisory text for detected arch patterns.
/// §COMMENTS LAW: the arch report lives in CHAT + kavach-db, never as a code
/// comment. This message MUST NOT prescribe writing a `// ARCH:` block.
#[must_use]
pub fn advise(file_path: &str, content: &str) -> Option<String> {
    let findings = detect(file_path, content);
    if findings.is_empty() {
        return None;
    }

    let mut msg = String::from("ARCH_ADVISORY:\n");
    for f in &findings {
        use std::fmt::Write;
        #[expect(
            clippy::let_underscore_must_use,
            reason = "writeln! to String always succeeds"
        )]
        let _ = writeln!(
            msg,
            "  Pattern '{}' (scope: {}) at L{}",
            f.keyword,
            f.scope.as_str(),
            f.line
        );
    }
    msg.push_str("\nRESEARCH via /arch skill — report in CHAT output, not as a code comment.\n");
    Some(msg)
}

/// Arch guard outcome.
///
/// §COMMENTS LAW: skill invocation alone unblocks the gate. The 9-field
/// `// ARCH:` code comment is no longer prescribed — the arch report is the
/// chat output + a `kavach db write --category decision` row, never a code
/// comment duplicated alongside.
#[must_use]
pub fn check(file_path: &str, content: &str, arch_skill_invoked: bool) -> ArchGuardOutcome {
    let findings = detect(file_path, content);

    if findings.is_empty() {
        return ArchGuardOutcome::Allow;
    }

    // Skill invoked this turn → allow. Single source of proof; no code comment.
    if arch_skill_invoked {
        return ArchGuardOutcome::Allow;
    }

    // Legacy compatibility only: a pre-existing 9-field `// ARCH:` comment in
    // older code still passes so historical files keep building. New code
    // satisfies the gate via the skill, never by writing a fresh block.
    if has_arch_comment(content) && count_arch_fields(content) >= REQUIRED_FIELDS {
        return ArchGuardOutcome::AllowWithComment;
    }

    let scopes: Vec<&str> = findings.iter().map(|f| f.scope.as_str()).collect();
    let unique_scopes: std::collections::HashSet<_> = scopes.iter().collect();
    let scope_list = unique_scopes
        .into_iter()
        .copied()
        .collect::<Vec<_>>()
        .join(", ");

    ArchGuardOutcome::Block(format!(
        "[ARCH_RESEARCH] Architectural patterns detected (scopes: {scope_list}) \
         -> invoke /arch skill OR rely on a prior arch decision row -> retry.\n\
         §COMMENTS LAW: do NOT add a `// ARCH:` code comment; the report is \
         the deliverable in chat, not a comment block in code."
    ))
}
