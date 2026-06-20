//! Attention dilution guard: warn when too many files processed in one pass.

use crate::gates::directive_cache::dyn_directive;

/// Threshold above which attention dilution warning fires.
/// Processing 12+ files degrades analysis depth — split into multi-pass.
const ATTENTION_THRESHOLD: i32 = 12;

/// Check if file read count just crossed attention threshold.
/// Fires once at threshold, then every 10 files to avoid spam.
pub(crate) fn check_attention(session: &kavach_session::SessionState) -> Option<String> {
    let count = session.subagent_files_read;
    if count < ATTENTION_THRESHOLD {
        return None;
    }
    // Fire at exact threshold, then at 22, 32, 42... (every 10 after)
    let at_threshold = count == ATTENTION_THRESHOLD;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "count > ATTENTION_THRESHOLD proven by guard above"
    )]
    let at_increment = count > ATTENTION_THRESHOLD && (count - ATTENTION_THRESHOLD) % 10 == 0;
    if !at_threshold && !at_increment {
        return None;
    }

    // Tag + counts literal; the remediation imperative is research-refreshed.
    let action = dyn_directive(
        "attention.dilution-remedy",
        "Analysis depth degrades beyond the threshold. Split into per-file analysis \
         passes, then synthesize: Pass 1 = analyze each file. Pass 2 = cross-file integration.",
    );
    Some(format!(
        "[ATTENTION_DILUTION]\n\
         files_processed: {count}\n\
         threshold: {ATTENTION_THRESHOLD}\n\
         {action}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_below_threshold() {
        let mut session = kavach_session::SessionState::default();
        session.subagent_files_read = 5;
        assert!(check_attention(&session).is_none());
    }

    #[test]
    fn test_at_threshold() {
        let mut session = kavach_session::SessionState::default();
        session.subagent_files_read = 12;
        let result = check_attention(&session);
        assert!(result.is_some());
        assert!(
            result
                .as_ref()
                .is_some_and(|r| r.contains("ATTENTION_DILUTION"))
        );
    }

    #[test]
    fn test_at_increment() {
        let mut session = kavach_session::SessionState::default();
        session.subagent_files_read = 22; // 12 + 10 = fires
        let result = check_attention(&session);
        assert!(result.is_some());
        assert!(result.as_ref().is_some_and(|r| r.contains("22")));
    }

    #[test]
    fn test_between_increments_silent() {
        let mut session = kavach_session::SessionState::default();
        session.subagent_files_read = 15; // between 12 and 22 — silent
        assert!(check_attention(&session).is_none());
    }
}
