use super::*;

#[test]
fn test_compress_short_context_unchanged() {
    let ctx = "[INTENT]\ntype: general\n";
    assert_eq!(compress(ctx, None), ctx);
}

#[test]
fn test_parse_sections_basic() {
    let ctx = "[INTENT]\ntype: general\n[KANBAN]\nrunnable: 3\n";
    let sections = parse_sections(ctx);
    assert_eq!(sections.len(), 2);
    assert!(sections[0].header.contains("[INTENT]"));
    assert!(sections[1].header.contains("[KANBAN]"));
}

#[test]
fn test_score_critical_section() {
    let section = super::Section { header: "[AUTONOMY_CONTRACT]\n".into(), content: "Act\n".into() };
    assert_eq!(score_section(&section), 100);
}

#[test]
fn test_deduplicate_lines() {
    let ctx = "line a\nline b\nline a\nline c\n";
    let out = deduplicate_lines(ctx);
    assert_eq!(out, "line a\nline b\nline c\n");
}
