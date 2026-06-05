//! Jaccard-similarity matching of query keywords against the Tailwind Plus index.

/// Jaccard similarity between query keywords and component keywords.
pub(super) fn jaccard(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let set_a: std::collections::HashSet<&str> = a.iter().map(String::as_str).collect();
    let set_b: std::collections::HashSet<&str> = b.iter().map(String::as_str).collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "keyword counts are typically ≤1000; precision loss negligible"
    )]
    #[expect(
        clippy::float_arithmetic,
        reason = "Jaccard coefficient math; no safe integer alternative"
    )]
    {
        intersection as f64 / union as f64
    }
}

/// Best-scoring component for `query_kw`, paired with its Jaccard score.
pub(super) fn find_best_match<'a>(
    components: &'a [serde_json::Value],
    query_kw: &[String],
) -> Option<(f64, &'a serde_json::Value)> {
    let mut best_score = 0.0_f64;
    let mut best: Option<&serde_json::Value> = None;
    for comp in components {
        let Some(arr) = comp.get("keywords").and_then(|v| v.as_array()) else {
            continue;
        };
        let comp_kw: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_lowercase))
            .collect();
        let score = jaccard(query_kw, &comp_kw);
        if score > best_score {
            best_score = score;
            best = Some(comp);
        }
    }
    best.map(|c| (best_score, c))
}
