// split: Adaptive trust model — loosen advisory gates as session count + acceptance rate climb.
//
// [RCA]
// symptom:    same friction at session 1 and session 1000; experienced users hit advisories that mature codebases shouldn't surface
// repro:      a project with 1000+ clean sessions still gets `format-in-loop` advisories on benign code
// why1:       gate severity is static — no per-project trust signal
// why2:       no telemetry on advisory acceptance / rejection rate
// why3:       invariant violated — friction should decay as evidence accumulates
// why4:       Anthropic Feb 2026 empirical study: 20% auto-approve at session 1 → 40% at 750+
// why5:       missing adaptive layer
// root_cause: no trust_score module
// class:      knowledge_gap
// blast_radius: every project consuming kavach gates
// research:   https://www.mindstudio.ai/blog/what-is-agent-harness-architecture-explained
//             https://www.nxcode.io/resources/news/what-is-harness-engineering-complete-guide-2026
// fix_strategy: pure-data trust scoring; host can read trust level + suppress P2 advisories above threshold

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate in kavach-rpc trust.rs; non_exhaustive => E0639"
)]
pub struct TrustInputs {
    pub session_count: u32,
    pub accepted_advisories: u32,
    pub rejected_advisories: u32,
    pub p0_blocks_in_last_30_sessions: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched cross-crate in kavach-rpc trust.rs; non_exhaustive => E0004"
)]
pub enum TrustLevel {
    /// Brand-new project. All gates active. Default for sessions < 10.
    Probationary,
    /// Some history. P0 strict; P1 strict; P2 advisory.
    Developing,
    /// Established project. P0 strict; P1 advisory; P2 silent.
    Established,
    /// Mature, low-incident project. P0 strict; P1/P2 silent.
    Mature,
}

impl TrustLevel {
    /// Should the host suppress this advisory tier?
    /// P0 is NEVER suppressed regardless of trust.
    #[must_use]
    pub const fn suppresses_p1(self) -> bool {
        matches!(self, Self::Mature)
    }
    #[must_use]
    pub const fn suppresses_p2(self) -> bool {
        matches!(self, Self::Established | Self::Mature)
    }
    #[must_use]
    pub const fn suppresses_p0(self) -> bool {
        false
    }
}

/// Compute trust level from session telemetry.
///
/// Thresholds derived from Anthropic Feb 2026 empirical curve (auto-approve % vs session count).
/// Recent P0 incidents reset trust to Developing — defensive depth on regression.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "acceptance_rate: division is intentional mathematical operation"
)]
pub fn classify(t: TrustInputs) -> TrustLevel {
    if t.p0_blocks_in_last_30_sessions > 0 {
        return TrustLevel::Developing;
    }
    let total_advisories = t.accepted_advisories.saturating_add(t.rejected_advisories);
    let acceptance = if total_advisories == 0 {
        // No advisory data yet; classify on session count alone.
        1.0
    } else {
        f64::from(t.accepted_advisories) / f64::from(total_advisories)
    };
    match (t.session_count, acceptance) {
        (s, _) if s < 10 => TrustLevel::Probationary,
        (s, a) if s < 100 || a < 0.5 => TrustLevel::Developing,
        (s, a) if s < 750 || a < 0.8 => TrustLevel::Established,
        _ => TrustLevel::Mature,
    }
}

/// Convenience: should an advisory at the given tier be surfaced under this trust?
#[must_use]
pub const fn should_surface(tier: AdvisoryTier, trust: TrustLevel) -> bool {
    match tier {
        AdvisoryTier::P0Block => true, // always surface
        AdvisoryTier::P1Advisory => !trust.suppresses_p1(),
        AdvisoryTier::P2Warning => !trust.suppresses_p2(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdvisoryTier {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(sess: u32, acc: u32, rej: u32, p0: u32) -> TrustInputs {
        TrustInputs {
            session_count: sess,
            accepted_advisories: acc,
            rejected_advisories: rej,
            p0_blocks_in_last_30_sessions: p0,
        }
    }

    #[test]
    fn new_project_probationary() {
        assert_eq!(classify(inputs(0, 0, 0, 0)), TrustLevel::Probationary);
        assert_eq!(classify(inputs(9, 0, 0, 0)), TrustLevel::Probationary);
    }

    #[test]
    fn small_history_developing() {
        assert_eq!(classify(inputs(50, 30, 5, 0)), TrustLevel::Developing);
    }

    #[test]
    fn medium_history_established() {
        assert_eq!(classify(inputs(200, 180, 20, 0)), TrustLevel::Established);
    }

    #[test]
    fn long_clean_history_mature() {
        assert_eq!(classify(inputs(1000, 950, 50, 0)), TrustLevel::Mature);
    }

    #[test]
    fn recent_p0_block_resets_to_developing() {
        let r = classify(inputs(1000, 950, 50, 1));
        assert_eq!(r, TrustLevel::Developing);
    }

    #[test]
    fn low_acceptance_rate_blocks_promotion() {
        // 1000 sessions but only 30% acceptance → stuck at Developing
        let r = classify(inputs(1000, 30, 70, 0));
        assert_eq!(r, TrustLevel::Developing);
    }

    #[test]
    fn p0_always_surfaces() {
        for t in [
            TrustLevel::Probationary,
            TrustLevel::Developing,
            TrustLevel::Established,
            TrustLevel::Mature,
        ] {
            assert!(should_surface(AdvisoryTier::P0Block, t));
        }
    }

    #[test]
    fn p2_silent_at_established_and_above() {
        assert!(should_surface(
            AdvisoryTier::P2Warning,
            TrustLevel::Probationary
        ));
        assert!(should_surface(
            AdvisoryTier::P2Warning,
            TrustLevel::Developing
        ));
        assert!(!should_surface(
            AdvisoryTier::P2Warning,
            TrustLevel::Established
        ));
        assert!(!should_surface(AdvisoryTier::P2Warning, TrustLevel::Mature));
    }

    #[test]
    fn p1_silent_only_at_mature() {
        assert!(should_surface(
            AdvisoryTier::P1Advisory,
            TrustLevel::Established
        ));
        assert!(!should_surface(
            AdvisoryTier::P1Advisory,
            TrustLevel::Mature
        ));
    }
}
