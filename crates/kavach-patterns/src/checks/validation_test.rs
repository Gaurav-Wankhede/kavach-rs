use super::*;

#[test]
fn test_is_inside_quotes() {
	assert!(!is_inside_quotes("hello world", 0));
	assert!(is_inside_quotes(r#""hello" world"#, 1));
	assert!(!is_inside_quotes(r#""hello" world"#, 8));
	assert!(is_inside_quotes("cmd 'data here'", 5));
	assert!(is_inside_quotes(r#"cmd "it's here""#, 8));
}
