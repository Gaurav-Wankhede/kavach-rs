use super::*;

#[test]
fn detects_rlo_override() {
    let hits = scan("rule: ignore safety\u{202E} but enforce safety");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].codepoint, '\u{202E}');
    assert_eq!(hits[0].line, 1);
}

#[test]
fn detects_zero_width_space_on_second_line() {
    let hits = scan("line1\nzero\u{200B}width");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].codepoint, '\u{200B}');
    assert_eq!(hits[0].line, 2);
}

#[test]
fn clean_content_passes() {
    assert!(scan("plain ASCII rules\nno hidden bytes").is_empty());
}

#[test]
fn detects_tag_block_codepoint() {
    let hits = scan("hello\u{E0041}");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].codepoint, '\u{E0041}');
    assert_eq!(hits[0].label, TAG_BLOCK_LABEL);
}

#[test]
fn detects_word_joiner_and_soft_hyphen() {
    let hits = scan("a\u{2060}b\u{00AD}c");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].codepoint, '\u{2060}');
    assert_eq!(hits[1].codepoint, '\u{00AD}');
}

#[test]
fn collects_multiple_hits_capped_at_max() {
    let payload: String = std::iter::repeat_n('\u{202E}', MAX_HITS + 3).collect();
    let hits = scan(&payload);
    assert_eq!(hits.len(), MAX_HITS);
}

#[test]
fn ai_config_path_recognised_at_filename_tail() {
    assert!(is_ai_config_path("/proj/.claude/agents/foo.md"));
    assert!(is_ai_config_path("/proj/CLAUDE.md"));
    assert!(is_ai_config_path("/proj/sub/CLAUDE.md"));
    assert!(is_ai_config_path("/proj/.cursorrules"));
    assert!(is_ai_config_path(
        "/proj/.github/copilot/copilot-instructions.md"
    ));
    assert!(is_ai_config_path("/proj/wrangler.toml"));
    assert!(is_ai_config_path("/proj/workers/api/wrangler.toml"));
    assert!(is_ai_config_path("/proj/rules/policy.mdc"));
}

#[test]
fn substring_lookalikes_not_flagged() {
    assert!(!is_ai_config_path("/proj/CLAUDE.md.bak"));
    assert!(!is_ai_config_path("/proj/notmywrangler.toml"));
    assert!(!is_ai_config_path("/proj/CLAUDE.md.swp"));
    assert!(!is_ai_config_path("/proj/myCLAUDE.md.txt"));
}

#[test]
fn non_config_path_not_flagged() {
    assert!(!is_ai_config_path("/proj/src/main.rs"));
    assert!(!is_ai_config_path("/proj/Cargo.toml"));
}

#[test]
fn block_message_lists_every_hit() {
    let hits = vec![
        BidiHit {
            line: 3,
            col: 7,
            codepoint: '\u{202E}',
            label: "U+202E RLO",
        },
        BidiHit {
            line: 5,
            col: 2,
            codepoint: '\u{2060}',
            label: "U+2060 WORD JOINER",
        },
    ];
    let msg = block_message("/proj/CLAUDE.md", &hits);
    assert!(msg.contains("0x202E"));
    assert!(msg.contains("0x2060"));
    assert!(msg.contains("Trojan Source"));
    assert!(msg.contains("2 hit(s)"));
}
