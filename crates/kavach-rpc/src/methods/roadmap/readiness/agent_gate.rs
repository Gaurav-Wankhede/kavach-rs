/// Check if a card is owner-gated (not actionable by an agent right now).
///
/// A card is OWNER-GATED iff its structured `owner_gated` field is `true` —
/// meaning it needs an external owner action an agent can never self-supply
/// (greenlight, prod deploy, live run, payment/CF secrets). This is a
/// SCHEDULING GATE (k8s 1.26 `schedulingGates` pattern): the dispatcher skips
/// the card entirely — exactly like an unmet dependency — so the autonomous
/// loop never claims work no agent can progress.
///
/// The gate reads the TYPED field, NEVER free-text body markers. The legacy
/// `AGENT_BLOCKED:` / `OWNER-GATED` / `OWNER-TASK` prose keywords are RETIRED
/// (owner directive 2026-06-13: state lives in a column, not card prose — the
/// same state-in-prose anti-pattern `priority`/`lane` already avoid). An
/// un-progressable card is either deleted or refined, never keyword-tagged.
#[must_use]
pub fn is_owner_gated(owner_gated: Option<bool>) -> bool {
    owner_gated.unwrap_or(false)
}
