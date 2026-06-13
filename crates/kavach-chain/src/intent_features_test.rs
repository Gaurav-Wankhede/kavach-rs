// Regression corpus for the destructive-feature false positives: a bare verb
// in prose ("remove the noise") must NOT classify destructive; verb+target
// pairs and shell idioms MUST. See roadmap
// unit.gate-noise.intent-classifier-precision.
use super::extract_features;

fn destructive(prompt: &str) -> bool {
    extract_features(prompt)
        .get("has_destructive")
        .and_then(kavach_dtree::Feature::as_bool)
        .expect("feature present")
}

#[test]
fn benign_prose_is_not_destructive() {
    // The exact false positive that motivated this card.
    assert!(!destructive(
        "We need to fix the loop. Remove all the noise from the gates so the harness works precisely"
    ));
    assert!(!destructive("Are you sure?"));
    assert!(!destructive("continue"));
    assert!(!destructive("what is the project status?"));
    // Word-boundary: "dropdown" must not trip "drop".
    assert!(!destructive("the dropdown is broken"));
    // Proximity window: target noun far from the verb is an edit, not a wipe.
    assert!(!destructive(
        "remove the unused import declaration near the top of the file"
    ));
}

#[test]
fn verb_plus_target_is_destructive() {
    assert!(destructive("delete the production database"));
    assert!(destructive("please remove the old backup files"));
    assert!(destructive("purge stale records"));
    assert!(destructive("wipe the data and start over"));
}

#[test]
fn shell_idioms_are_destructive() {
    assert!(destructive("run rm -rf target/ to clean"));
    assert!(destructive("drop table users"));
    assert!(destructive("git reset --hard and force push"));
}

#[test]
fn extracts_debug_features() {
    let features = extract_features("fix this bug in the handler");
    assert_eq!(
        features
            .get("has_debug")
            .and_then(kavach_dtree::Feature::as_bool),
        Some(true)
    );
    assert_eq!(
        features
            .get("has_implement")
            .and_then(kavach_dtree::Feature::as_bool),
        Some(false)
    );
}

#[test]
fn extracts_deploy_features() {
    let features = extract_features("deploy to production");
    assert_eq!(
        features
            .get("has_deploy")
            .and_then(kavach_dtree::Feature::as_bool),
        Some(true)
    );
}

#[test]
fn extracts_word_count() {
    let features = extract_features("one two three");
    assert_eq!(
        features
            .get("word_count")
            .and_then(kavach_dtree::Feature::as_numeric),
        Some(3.0)
    );
}
