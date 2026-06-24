// Parallel co-pilot serving for the NanoLM helper: fan out the expert-policies
// over the BM25 corpus with tokio, then aggregate to a best-tradeoff answer.
//
// The aggregation (tradeoff = grounding*confidence - risk) and the expert-policy
// enum are inlined here, NOT imported from kavach-nlm: kavach-nlm depends on this
// crate (the in-process RPC client), so the edge cannot reverse. kavach-nlm holds
// the canonical worker-side copy; this is the daemon-side serving copy.
use std::sync::Arc;

use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::nlm_query_docs;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

/// The expert-policies the serving path fans out — external Kavach skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExpertPolicy {
    /// Scores an action against the cited best-vs-worst-practice corpus.
    BestPracticeScorer,
    /// Flags the security posture of an action (fail-closed bias).
    CybersecAdvisor,
    /// Forces a live fetch of the latest authoritative source.
    FetchPreciseLatest,
    /// Weighs parallel multi-task tradeoffs and picks the best fit.
    TradeoffSolver,
    /// Classifies the query intent to route downstream work.
    IntentRouter,
}

impl ExpertPolicy {
    const ALL: [Self; 5] = [
        Self::BestPracticeScorer,
        Self::CybersecAdvisor,
        Self::FetchPreciseLatest,
        Self::TradeoffSolver,
        Self::IntentRouter,
    ];

    /// The cybersec lens: intrinsic risk of this expert's domain in `[0, 1]`.
    const fn risk(self) -> f32 {
        match self {
            Self::CybersecAdvisor => 0.4,
            Self::FetchPreciseLatest => 0.2,
            _ => 0.1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct AdviseParams {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct ScoredAdvice {
    pub policy: ExpertPolicy,
    pub advice: String,
    pub grounding: f32,
    pub confidence: f32,
    pub risk: f32,
    pub tradeoff_score: f32,
}

impl ScoredAdvice {
    /// The tradeoff score: reward grounding and confidence, penalize risk.
    #[expect(
        clippy::float_arithmetic,
        reason = "the tradeoff score is an f32 combination by construction"
    )]
    fn scored(policy: ExpertPolicy, advice: String, grounding: f32, confidence: f32) -> Self {
        let risk = policy.risk();
        Self {
            policy,
            advice,
            grounding,
            confidence,
            risk,
            tradeoff_score: grounding.mul_add(confidence, -risk),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct AdviseResult {
    pub best: ScoredAdvice,
    pub dissents: Vec<ScoredAdvice>,
}

/// Run ONE expert-policy: retrieve over the BM25 corpus, derive a [`ScoredAdvice`].
///
/// Grounding is `+1` when the corpus returns a hit (the advice is cited) and
/// `-1` when it does not (an uncited claim — never-trust-weights penalizes it).
/// Confidence scales with the hit count, saturating at the requested limit.
#[expect(
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    reason = "hit counts are tiny; f32 precision is ample for a confidence ratio"
)]
async fn run_expert(
    db: &Surreal<Any>,
    policy: ExpertPolicy,
    query: &str,
    limit: usize,
) -> Result<ScoredAdvice, ErrorObjectOwned> {
    let hits = nlm_query_docs(db, query, limit)
        .await
        .map_err(surreal_to_rpc)?;
    let advice = hits.first().map_or_else(
        || format!("{policy:?}: no cited source — refuse to advise from weights, fetch first"),
        |h| format!("{policy:?}: per {}, {}", h.source_url, h.heading),
    );
    let grounding = if hits.is_empty() { -1.0 } else { 1.0 };
    let confidence = if limit == 0 {
        0.0
    } else {
        (hits.len() as f32 / limit as f32).min(1.0)
    };
    Ok(ScoredAdvice::scored(policy, advice, grounding, confidence))
}

/// `nlm.advise`: fan out every expert-policy in parallel over the corpus and
/// aggregate to the best-tradeoff co-pilot answer (winner + ranked dissents).
///
/// # Errors
/// Returns `ErrorObjectOwned` if the query is empty, a retrieval fails, or an
/// expert task panics.
pub async fn advise(state: &AppState, p: AdviseParams) -> Result<AdviseResult, ErrorObjectOwned> {
    let query = p.query.trim().to_owned();
    if query.is_empty() {
        return Err(ErrorObjectOwned::owned(
            -32010,
            "nlm.advise needs a non-empty query",
            None::<()>,
        ));
    }
    let limit = p.limit.unwrap_or(5).clamp(1, 50);

    let mut set = tokio::task::JoinSet::new();
    for policy in ExpertPolicy::ALL {
        let db: Arc<Surreal<Any>> = Arc::clone(&state.db);
        let q = query.clone();
        set.spawn(async move { run_expert(&db, policy, &q, limit).await });
    }

    let mut scored = Vec::new();
    while let Some(joined) = set.join_next().await {
        let advice = joined.map_err(|e| {
            ErrorObjectOwned::owned(-32011, format!("expert task panicked: {e}"), None::<()>)
        })??;
        scored.push(advice);
    }

    scored.sort_by(|a, b| {
        b.tradeoff_score
            .partial_cmp(&a.tradeoff_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut iter = scored.into_iter();
    let best = iter.next().ok_or_else(|| {
        ErrorObjectOwned::owned(-32012, "no candidate advisories to aggregate", None::<()>)
    })?;
    Ok(AdviseResult {
        best,
        dissents: iter.collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::{ExpertPolicy, ScoredAdvice};

    fn rank(mut v: Vec<ScoredAdvice>) -> Vec<ScoredAdvice> {
        v.sort_by(|a, b| {
            b.tradeoff_score
                .partial_cmp(&a.tradeoff_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v
    }

    #[test]
    fn grounded_low_risk_outranks_uncited_confident() {
        // grounded best-practice: +1 grounding, 0.9 conf, 0.1 risk -> 0.8
        let good = ScoredAdvice::scored(
            ExpertPolicy::BestPracticeScorer,
            "cited".to_owned(),
            1.0,
            0.9,
        );
        // uncited cybersec: -1 grounding, 1.0 conf, 0.4 risk -> -1.4
        let bad = ScoredAdvice::scored(
            ExpertPolicy::CybersecAdvisor,
            "uncited".to_owned(),
            -1.0,
            1.0,
        );
        let ranked = rank(vec![bad, good]);
        assert_eq!(
            ranked.first().map(|s| s.policy),
            Some(ExpertPolicy::BestPracticeScorer)
        );
        assert!(ranked.first().is_some_and(|s| s.tradeoff_score > 0.0));
        assert!(ranked.get(1).is_some_and(|s| s.tradeoff_score < 0.0));
    }

    #[test]
    fn risk_breaks_ties_against_riskier_policy() {
        // Equal grounding+confidence; cybersec (risk 0.4) must rank below fetch (0.2).
        let cyber = ScoredAdvice::scored(ExpertPolicy::CybersecAdvisor, "c".to_owned(), 1.0, 0.5);
        let fetch =
            ScoredAdvice::scored(ExpertPolicy::FetchPreciseLatest, "f".to_owned(), 1.0, 0.5);
        let ranked = rank(vec![cyber, fetch]);
        assert_eq!(
            ranked.first().map(|s| s.policy),
            Some(ExpertPolicy::FetchPreciseLatest)
        );
    }
}
