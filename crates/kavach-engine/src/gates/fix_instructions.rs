use std::fmt::Write as _;

use kavach_patterns::AntiProdResult;

use super::fix_actions::fix_action;

/// Generate structured fix instructions from antiprod violations.
pub(crate) fn generate_fix_instructions(violations: &[AntiProdResult], file_path: &str) -> String {
    let max_severity = violations
        .iter()
        .map(|v| severity_label(v.level))
        .min()
        .unwrap_or("P3");

    let mut out = format!(
        "[FIX_REQUIRED]\nfile: {file_path}\nseverity: {max_severity}\ncount: {}",
        violations.len()
    );

    for (i, v) in violations.iter().enumerate() {
        let level = severity_label(v.level);
        let (action, instruction) = fix_action(v.code, &v.match_text);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "i bounded by violations.len()"
        )]
        let fix_num = i + 1;
        writeln!(
            out,
            "\n\n[FIX:{fix_num}]\nlevel: {level}\ncode: {}\nmatch: {}\n\
             action: {action}\ninstruction: {instruction}",
            v.code, v.match_text,
        )
        .ok();
    }
    out
}

const fn severity_label(level: kavach_patterns::AntiProdLevel) -> &'static str {
    use kavach_patterns::AntiProdLevel::{P0MockData, P1ProdLeak, P2ErrorBlind, P3TypeLoose};
    match level {
        P0MockData => "P0",
        P1ProdLeak => "P1",
        P2ErrorBlind => "P2",
        P3TypeLoose => "P3",
    }
}
