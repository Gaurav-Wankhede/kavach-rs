use super::node::TreeNode;
use super::query::Query;

/// A scored match between a query and a tree node. Higher = better match.
/// Zero means no signal — the matcher filters these out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639"
)]
pub struct Score(pub u32);

impl Score {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn is_nonzero(self) -> bool {
        self.0 > 0
    }
}

const WEIGHT_FILE_PATTERN: u32 = 40;
const WEIGHT_KEYWORD: u32 = 10;
const WEIGHT_SUMMARY_TOKEN: u32 = 2;
const WEIGHT_INTENT_TITLE: u32 = 20;

/// Compute a query-vs-node score using weighted signals.
///
/// File patterns dominate (structural signal), keywords next, summary token
/// overlap is softest. Intent-in-title adds a small boost.
/// `graph_boost` is pre-computed by `rag::graph_boost::compute_graph_boost` — zero
/// DB coupling in the scorer itself.
#[must_use]
pub fn score_node(node: &TreeNode, query: &Query) -> Score {
    score_node_with_boost(node, query, 0)
}

/// Score with an explicit graph boost — used when graph connection is available.
#[must_use]
pub fn score_node_with_boost(node: &TreeNode, query: &Query, graph_boost: u32) -> Score {
    let mut total: u32 = 0;
    total = total.saturating_add(file_pattern_score(node, query));
    total = total.saturating_add(keyword_score(node, query));
    total = total.saturating_add(summary_token_score(node, query));
    total = total.saturating_add(intent_title_score(node, query));
    total = total.saturating_add(graph_boost);
    Score(total)
}

fn file_pattern_score(node: &TreeNode, query: &Query) -> u32 {
    let path = query.file_path();
    let hits = node
        .file_patterns
        .iter()
        .filter(|pat| path_matches(pat, path))
        .count();
    u32::try_from(hits)
        .unwrap_or(u32::MAX)
        .saturating_mul(WEIGHT_FILE_PATTERN)
}

fn keyword_score(node: &TreeNode, query: &Query) -> u32 {
    let tokens = query.tokens();
    let hits = node
        .keywords
        .iter()
        .map(|k| k.to_lowercase())
        .filter(|kw| {
            // Multi-word keyword: all words must appear in query tokens
            let kw_parts: Vec<&str> = kw.split_whitespace().collect();
            if kw_parts.len() > 1 {
                kw_parts.iter().all(|part| tokens.iter().any(|t| t == part))
            } else {
                // Single-word: exact token match
                tokens.iter().any(|t| t == kw)
            }
        })
        .count();
    u32::try_from(hits)
        .unwrap_or(u32::MAX)
        .saturating_mul(WEIGHT_KEYWORD)
}

fn summary_token_score(node: &TreeNode, query: &Query) -> u32 {
    let summary_lower = node.summary.to_lowercase();
    let hits = query
        .tokens()
        .iter()
        .filter(|tok| tok.len() >= 4 && summary_lower.contains(tok.as_str()))
        .count();
    u32::try_from(hits)
        .unwrap_or(u32::MAX)
        .saturating_mul(WEIGHT_SUMMARY_TOKEN)
}

fn intent_title_score(node: &TreeNode, query: &Query) -> u32 {
    if query.intent().is_empty() {
        return 0;
    }
    let intent_lower = query.intent().to_lowercase();
    if node.title.to_lowercase().contains(&intent_lower) {
        WEIGHT_INTENT_TITLE
    } else {
        0
    }
}

/// Minimal glob matcher: supports `*` as wildcard between literal segments.
fn path_matches(pattern: &str, path: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    match parts.as_slice() {
        [] => false,
        [only] => path == *only,
        [first, rest @ ..] => match_segments(first, rest, path),
    }
}

fn match_segments(first: &str, rest: &[&str], path: &str) -> bool {
    if !path.starts_with(first) {
        return false;
    }
    let mut cursor = first.len();
    let last_index = rest.len().saturating_sub(1);
    for (i, segment) in rest.iter().enumerate() {
        if segment.is_empty() {
            continue;
        }
        let Some(haystack) = path.get(cursor..) else {
            return false;
        };
        if i == last_index {
            return haystack.ends_with(segment);
        }
        match haystack.find(segment) {
            Some(pos) => {
                cursor = cursor.saturating_add(pos).saturating_add(segment.len());
            }
            None => return false,
        }
    }
    true
}
