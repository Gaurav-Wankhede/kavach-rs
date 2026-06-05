//! Detect bidirectional, zero-width, and tag-block Unicode injections in AI config files.
//!
//! rustc 1.56.1+ lints bidi codepoints in `.rs` source per CVE-2021-42574. This guard
//! covers `.md`, `.toml`, `.mdc`, `.cursorrules` files that AI assistants load as
//! instructions but compilers never see — the surface targeted by Rules File Backdoor
//! and 2025–2026 invisible-Unicode prompt-injection research.
//!
//! Codepoint coverage spans three threat eras:
//!   1. CVE-2021-42574 bidi (U+202A–U+202E, U+2066–U+2069)
//!   2. Zero-width / invisible formatters (U+200B–U+200D, U+2060, U+FEFF, U+00AD,
//!      U+034F, U+180E)
//!   3. Tag block U+E0000–U+E007F (Pillar 2025, AWS 2026 — ASCII-equivalent invisible
//!      smuggling channel)
//!
//! API contract: `scan` returns ALL hits up to `MAX_HITS` so the defender sees every
//! codepoint, not just the first. `is_ai_config_path` matches on filename tail via
//! `Path::file_name()` to avoid `.bak`/`.swp` false-positives and nested-workspace
//! false-negatives.
//!
//! SOURCE: <https://blog.rust-lang.org/2021/11/01/cve-2021-42574>/ (rustc lint scope)
//! SOURCE: <https://www.trojansource.codes>/ (CVE-2021-42574, Cambridge 2021)
//! SOURCE: <https://www.pillar.security/blog/new-vulnerability-in-github-copilot-and-cursor-how-hackers-can-weaponize-code-agents> (Pillar Security, Mar 2025)
//! SOURCE: <https://aws.amazon.com/blogs/security/defending-llm-applications-against-unicode-character-smuggling>/ (AWS, 2026 — tag block U+E0000–U+E007F)
//! SOURCE: <https://idanhabler.medium.com/hiding-in-plain-sight-weaponizing-invisible-unicode-to-attack-llms-f9033865ec10> (zero-width binary smuggling, 2025)
//! SOURCE: <https://doc.rust-lang.org/std/path/struct.Path.html> (`Path::file_name` idiom)

use std::fmt::Write;
use std::path::Path;

const MAX_HITS: usize = 5;

const NAMED_DANGEROUS: &[(char, &str)] = &[
    ('\u{202A}', "U+202A LRE (LEFT-TO-RIGHT EMBEDDING)"),
    ('\u{202B}', "U+202B RLE (RIGHT-TO-LEFT EMBEDDING)"),
    ('\u{202C}', "U+202C PDF (POP DIRECTIONAL FORMATTING)"),
    ('\u{202D}', "U+202D LRO (LEFT-TO-RIGHT OVERRIDE)"),
    ('\u{202E}', "U+202E RLO (RIGHT-TO-LEFT OVERRIDE)"),
    ('\u{2066}', "U+2066 LRI (LEFT-TO-RIGHT ISOLATE)"),
    ('\u{2067}', "U+2067 RLI (RIGHT-TO-LEFT ISOLATE)"),
    ('\u{2068}', "U+2068 FSI (FIRST STRONG ISOLATE)"),
    ('\u{2069}', "U+2069 PDI (POP DIRECTIONAL ISOLATE)"),
    ('\u{200B}', "U+200B ZWSP (ZERO-WIDTH SPACE)"),
    ('\u{200C}', "U+200C ZWNJ (ZERO-WIDTH NON-JOINER)"),
    ('\u{200D}', "U+200D ZWJ (ZERO-WIDTH JOINER)"),
    ('\u{2060}', "U+2060 WORD JOINER"),
    ('\u{FEFF}', "U+FEFF BOM/ZWNBSP"),
    ('\u{00AD}', "U+00AD SOFT HYPHEN"),
    ('\u{034F}', "U+034F COMBINING GRAPHEME JOINER"),
    ('\u{180E}', "U+180E MONGOLIAN VOWEL SEPARATOR"),
    ('\u{2028}', "U+2028 LINE SEPARATOR"),
    ('\u{2029}', "U+2029 PARAGRAPH SEPARATOR"),
];

const TAG_BLOCK_LABEL: &str = "TAG BLOCK (U+E0000-U+E007F invisible-ASCII smuggling channel)";

const AI_CONFIG_EXACT_NAMES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    "GEMINI.md",
    "wrangler.toml",
    ".cursorrules",
    "copilot-instructions.md",
];

const AI_CONFIG_DIR_FRAGMENTS: &[&str] = &[
    "/.claude/",
    "/.cursor/",
    "/.continue/",
    "/.copilot/",
    "/.github/copilot/",
];

const AI_CONFIG_EXTS: &[&str] = &[".mdc", ".cursorrules"];

fn is_dangerous(ch: char) -> Option<&'static str> {
    if let Some((_, label)) = NAMED_DANGEROUS.iter().find(|(c, _)| *c == ch) {
        return Some(*label);
    }
    let cp = ch as u32;
    if (0xE0000..=0xE007F).contains(&cp) {
        return Some(TAG_BLOCK_LABEL);
    }
    None
}

#[must_use]
pub fn is_ai_config_path(path: &str) -> bool {
    if AI_CONFIG_DIR_FRAGMENTS.iter().any(|f| path.contains(f)) {
        return true;
    }
    let Some(name_os) = Path::new(path).file_name() else {
        return false;
    };
    let Some(name) = name_os.to_str() else {
        return false;
    };
    if AI_CONFIG_EXACT_NAMES.contains(&name) {
        return true;
    }
    AI_CONFIG_EXTS.iter().any(|ext| name.ends_with(ext))
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BidiHit {
    pub line: usize,
    pub col: usize,
    pub codepoint: char,
    pub label: &'static str,
}

#[must_use]
pub fn scan(content: &str) -> Vec<BidiHit> {
    let mut hits = Vec::new();
    let mut line = 1usize;
    let mut col = 1usize;
    for ch in content.chars() {
        if let Some(label) = is_dangerous(ch)
            && hits.len() < MAX_HITS
        {
            hits.push(BidiHit {
                line,
                col,
                codepoint: ch,
                label,
            });
        }
        if ch == '\n' {
            line = line.saturating_add(1);
            col = 1;
        } else {
            col = col.saturating_add(1);
        }
    }
    hits
}

#[must_use]
pub fn block_message(path: &str, hits: &[BidiHit]) -> String {
    let mut out = format!(
        "BLOCKED: bidirectional/zero-width/tag-block Unicode in AI-config file: {path}\n\
         {} hit(s) detected (showing up to {MAX_HITS}):\n",
        hits.len()
    );
    for hit in hits {
        writeln!(
            out,
            "  line {}, col {}: {} ({:#06X})",
            hit.line, hit.col, hit.label, hit.codepoint as u32
        )
        .ok();
    }
    out.push_str(
        "\nTrojan Source / Rules File Backdoor / Invisible-Unicode prompt injection.\n\
         Hidden codepoints redirect AI behaviour while staying invisible in the editor.\n\
         FIX: Remove every codepoint listed. If genuinely needed (test fixture), move \
         to a non-AI-config file or escape as \\u{XXXX} inside a quoted string.",
    );
    out
}

#[cfg(test)]
mod tests {
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
}
