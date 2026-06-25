use super::*;

#[test]
fn test_intent() {
	let r = classify_intent("fix bug");
	assert!(r == "debug" || r == "fix", "got {r}");
	assert_eq!(classify_intent("hello"), "general");
}

#[test]
fn test_agent() {
	assert!(is_valid_agent("ceo"));
	assert!(!is_valid_agent("xyz"));
}
