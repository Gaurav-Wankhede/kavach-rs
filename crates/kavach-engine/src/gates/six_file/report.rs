// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
//
// `tier` groups missing prefixes by required tier; this hub formats the
// `[SIX_FILE_BLOCK]` header, coverage summary, tier sections, and draft guide.
mod tier;
#[cfg(test)]
#[path = "report_test.rs"]
#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
use kavach_types::WitnessResult;
use std::fmt::Write as _;
use super::auto_draft;
#[must_use]
pub(crate) fn format_block(result: &WitnessResult) -> String {
    let mut buf = String::new();
    buf.push_str("[SIX_FILE_BLOCK]\n\n");
    write!(
        buf,
        "Project: {}\nTier: {}\n",
        result.project_slug,
        result.tier.as_str()
    )
    .ok();
    writeln!(
        buf,
        "Spec coverage: {}/{} required artifacts present.\n",
        result.present, result.required
    )
    .ok();
    if result.missing.is_empty() {
        buf.push_str("Status: CLEAR ✓\n");
        return buf;
    }
    tier::append_sections(&mut buf, &result.missing);
    buf.push_str("How to draft:\n\n");
    for m in &result.missing {
        writeln!(buf, "{}\n", auto_draft::draft_block(m)).ok();
    }
    buf
}
