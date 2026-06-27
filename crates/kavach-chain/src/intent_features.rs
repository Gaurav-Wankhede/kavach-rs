// SOURCE: https://docs.rs/linfa-trees/ — feature extraction for classification
use kavach_dtree::FeatureSet;
/// Extract boolean features from a prompt for decision tree classification.
#[must_use]
pub fn extract_features(prompt: &str) -> FeatureSet {
    let lower = prompt.to_lowercase();
    FeatureSet::new()
        .with_bool("has_destructive", destructive::has_destructive(&lower))
        .with_bool("has_deploy", has_deploy(&lower))
        .with_bool("has_security", has_security(&lower))
        .with_bool("has_debug", has_debug(&lower))
        .with_bool("has_refactor", has_refactor(&lower))
        .with_bool("has_implement", has_implement(&lower))
        .with_bool("has_memory", has_memory(&lower))
        .with_numeric("word_count", {
            #[expect(clippy::cast_precision_loss, reason = "word_count <1M fits f64")]
            let count = prompt.split_whitespace().count() as f64;
            count
        })
}
mod destructive;
fn has_deploy(s: &str) -> bool {
    contains_any(
        s,
        &["deploy", "release", "publish", "production", "go live"],
    )
}
fn has_security(s: &str) -> bool {
    contains_any(
        s,
        &[
            "security",
            "auth",
            "encrypt",
            "vulnerability",
            "cve",
            "attack",
        ],
    )
}
fn has_debug(s: &str) -> bool {
    contains_any(
        s,
        &[
            "fix",
            "bug",
            "error",
            "debug",
            "broken",
            "not working",
            "crash",
            "find",
            "discover",
            "locate",
            "trace",
            "investigate",
            "diagnose",
        ],
    )
}
fn has_refactor(s: &str) -> bool {
    contains_any(
        s,
        &["refactor", "restructure", "clean up", "improve", "optimize"],
    )
}
fn has_implement(s: &str) -> bool {
    contains_any(
        s,
        &["implement", "create", "build", "add", "develop", "write"],
    )
}
fn has_memory(s: &str) -> bool {
    contains_any(
        s,
        &[
            "memory bank",
            "update memory",
            "remember this",
            "save to memory",
        ],
    )
}
fn contains_any(s: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| s.contains(p))
}
#[cfg(test)]
#[path = "intent_features_test.rs"]
#[cfg(test)]
#[path = "intent_features_test.rs"]
mod tests;
