//! Laziness guard — blocks the agent from dressing LABOR up as a DIRECTION question.
//!
//! The recurring loophole: when two paths differ in EFFORT (not in
//! genuine direction), the agent surfaces an `AskUserQuestion` whose
//! `(Recommended)` option is the LOWER-EFFORT one ("leave as-is", "check back
//! later", "skip the rebuild"), offloading doable work onto the user as a fake
//! "choice". Per the global division-of-labor rule, the user decides DIRECTION;
//! the agent does ALL the labor. Recommending the lazy option violates that.
//!
//! This is a `PreToolUse:AskUserQuestion` detector. It is a P0 hard block (deny the
//! tool call) — not an advisory — because the false-positive set is bounded: it
//! fires ONLY when a `(Recommended)`-tagged option ALSO carries a low-effort
//! marker AND a sibling option is the higher-effort do-the-work path. A genuine
//! direction question (auth method, library choice, design A vs B) carries no
//! effort-asymmetry markers and is never flagged.
/// Low-effort / labor-deferral markers. An option carrying one of these is the
/// "don't do the work now" path. Matched case-insensitively against the option's
/// label + description.
const LAZY_MARKERS: &[&str] = &[
    "leave as-is",
    "leave as is",
    "leave it",
    "as-is",
    "as is",
    "check back later",
    "come back later",
    "later",
    "defer",
    "skip the",
    "skip it",
    "skip rebuild",
    "no rebuild",
    "don't rebuild",
    "do nothing",
    "leave the",
    "stop now",
    "leave stale",
    "ship a summary",
    "document it instead",
    "wait for",
    "minimal change only",
    "punt",
    // Deferral synonyms (marker-set evasion close): an evader can dodge the block
    // with an unmarked procrastination verb. These are unambiguous in this context.
    "postpone",
    "revisit later",
    "set aside",
    "shelve",
    "park it",
    "park for",
    "kick down",
    "table it",
    "table for",
    "hold off",
];
/// Higher-effort / do-the-work markers — the option that actually does the labor.
/// Presence of one of these as a SIBLING confirms the choice is an effort split,
/// not a direction split.
const WORK_MARKERS: &[&str] = &[
    "rebuild",
    "full ",
    "canonical",
    "fix all",
    "fix the",
    "implement",
    "do the work",
    "complete",
    "finish",
    "build the",
    "triage",
    "purge",
    "run the",
    "the proper",
    "the correct",
    "end-to-end",
    "thorough",
    // Additional do-the-work verbs (marker-set completeness): unambiguous work-class
    // terms an evader might use to phrase the do-work sibling without a marked word.
    "refactor",
    "migrate",
    "port ",
    "integrate",
    "consolidate",
    "the full",
    "properly",
];
/// The `(Recommended)` suffix the agent appends to its recommended option per the
/// `AskUserQuestion` tool contract.
const RECOMMENDED: &str = "recommended";
/// One option's text, lowercased for matching.
struct Opt {
    text: String,
    recommended: bool,
}
fn any_marker(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|m| text.contains(m))
}
/// Parse the `AskUserQuestion` `tool_input` JSON into PER-QUESTION option groups
/// (one inner `Vec<Opt>` per question), each option being label + description
/// joined and lowercased, with its recommended flag. Keeping options grouped by
/// question is what lets the detector evaluate each question independently and
/// never cross-pair options from different questions.
fn extract_option_groups(tool_input: &serde_json::Value) -> Vec<Vec<Opt>> {
    let mut groups = Vec::new();
    let Some(questions) = tool_input.get("questions").and_then(|q| q.as_array()) else {
        return groups;
    };
    for q in questions {
        let Some(opts) = q.get("options").and_then(|o| o.as_array()) else {
            continue;
        };
        let mut group = Vec::new();
        for o in opts {
            let label = o.get("label").and_then(|v| v.as_str()).unwrap_or("");
            let desc = o.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let joined = format!("{label} {desc}").to_lowercase();
            let recommended = joined.contains(RECOMMENDED);
            group.push(Opt {
                text: joined,
                recommended,
            });
        }
        groups.push(group);
    }
    groups
}
/// Tokens that mark a question's subject as a CONCRETE EXTERNAL artifact whose
/// correct value lives on the internet (docs / source / RFC), not in the user's
/// head: a library, an API, a version, a flag, an algorithm. A question framed
/// around these is RESEARCHABLE — the agent must `WebSearch` the authoritative
/// source, not ask the user to recall it.
const RESEARCHABLE_SUBJECT: &[&str] = &[
    "library",
    "crate",
    "package",
    "dependency",
    "api ",
    "endpoint",
    "signature",
    "function name",
    "method name",
    "flag",
    "cli option",
    "command-line",
    "version",
    "semver",
    "compatible",
    "supported",
    "algorithm",
    "data structure",
    "protocol",
    "rfc",
    "spec",
    "syntax",
    "config key",
    "env var",
    "environment variable",
    "default value",
    "which method",
    "which function",
    "correct way to",
    "right way to",
];
/// Question phrasings that ask for a FACTUAL/TECHNICAL answer ("which / what is")
/// rather than a DIRECTION tradeoff. Combined with a researchable subject, these
/// mark a question the internet answers definitively.
const FACTUAL_QUESTION_FORMS: &[&str] = &[
    "which ",
    "what is the",
    "what's the",
    "whats the",
    "how do i",
    "how to",
    "does it support",
    "is it compatible",
    "what version",
    "what flag",
    "what api",
];
/// Markers of a GENUINE direction / authorization decision — the user's call,
/// never researchable. Their presence SUPPRESSES the researchable-question flag
/// so a real tradeoff ("which approach fits OUR latency budget", "priority",
/// "should I push/delete/deploy") is never nudged toward `WebSearch`.
const DIRECTION_OR_AUTH_MARKERS: &[&str] = &[
    "priority",
    "which approach",
    "tradeoff",
    "trade-off",
    "design a",
    "architecture",
    "scope",
    "push",
    "delete",
    "deploy",
    "send",
    "merge",
    "release",
    "our ",
    "this project",
    "business",
    "user-facing",
    "prefer",
    "preference",
    "irreversible",
];
/// Return `Some(advisory)` when an `AskUserQuestion` poses a RESEARCHABLE
/// factual/technical question.
///
/// "Researchable" = a library / API / version / flag / algorithm choice
/// answerable by `WebSearch`, with NO genuine direction or authorization marker.
/// `None` for real direction/authorization questions. ADVISORY tier (not a hard
/// block): the false-positive surface is wider than the effort-split case, and the
/// correct response is "research first", a nudge — not a deny.
#[must_use]
pub fn detect_researchable_question(tool_input: &serde_json::Value) -> Option<String> {
    let questions = tool_input.get("questions").and_then(|q| q.as_array())?;
    for q in questions {
        let stem = q
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        // Fold the option text in too: the subject often lives in the options
        // ("Reqwest" / "Hyper") while the stem only says "which".
        let opts_text = q
            .get("options")
            .and_then(|o| o.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|o| {
                        let l = o.get("label").and_then(|v| v.as_str()).unwrap_or("");
                        let d = o.get("description").and_then(|v| v.as_str()).unwrap_or("");
                        format!("{l} {d}")
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
            .to_lowercase();
        let haystack = format!("{stem} {opts_text}");
        // A genuine direction/authorization question is the user's call — never nudge it.
        if any_marker(&haystack, DIRECTION_OR_AUTH_MARKERS) {
            continue;
        }
        let researchable_subject = any_marker(&haystack, RESEARCHABLE_SUBJECT);
        let factual_form = any_marker(&stem, FACTUAL_QUESTION_FORMS);
        if researchable_subject && factual_form {
            return Some(researchable_advisory());
        }
    }
    None
}
fn researchable_advisory() -> String {
    "[RESEARCH_FIRST] This AskUserQuestion asks a FACTUAL/TECHNICAL question \
     (a library / API / version / flag / algorithm choice) whose authoritative \
     answer lives on the internet — official docs, the dependency's own \
     --help/source, the upstream RFC/issue — NOT in the user's head. Do NOT ask: \
     WebSearch the current authoritative source, corroborate across 2+ references \
     (2026), adopt the precise contract, and sync the finding to the kavach DB. \
     Ask the user ONLY for a genuine DIRECTION tradeoff or IRREVERSIBLE \
     authorization (global CLAUDE.md §research_before_building / §act_not_narrate)."
        .to_owned()
}
/// Return `Some(reason)` when an `AskUserQuestion` recommends the lower-effort
/// path over a higher-effort do-the-work sibling — the labor-as-direction
/// loophole. `None` for genuine direction questions.
///
/// Each question is evaluated INDEPENDENTLY: a lazy-recommended option in one
/// question may only be paired with a do-the-work sibling from the SAME question.
/// Flattening across questions would let a pure-direction Q1 option cross-pair
/// with a pure-effort Q2 option (a cross-question false hit) and, symmetrically,
/// let an evader split the two halves across questions to dodge the block.
#[must_use]
pub fn detect_lazy_recommendation(tool_input: &serde_json::Value) -> Option<String> {
    let groups = extract_option_groups(tool_input);
    if groups.iter().any(|opts| group_is_lazy_recommendation(opts)) {
        return Some(block_reason());
    }
    None
}
/// True when, WITHIN one question's options, a `(Recommended)` option carries a
/// LAZY marker and a non-recommended sibling carries a WORK marker.
fn group_is_lazy_recommendation(opts: &[Opt]) -> bool {
    if opts.len() < 2 {
        return false;
    }
    let recommended_is_lazy = opts
        .iter()
        .any(|o| o.recommended && any_marker(&o.text, LAZY_MARKERS));
    // Confirm the split is EFFORT, not direction: a sibling must be the do-work path.
    let sibling_is_work = opts
        .iter()
        .any(|o| !o.recommended && any_marker(&o.text, WORK_MARKERS));
    recommended_is_lazy && sibling_is_work
}
fn block_reason() -> String {
    "[LAZINESS_BLOCK] This AskUserQuestion recommends the LOWER-EFFORT option \
     (a 'leave as-is / skip / later / defer' choice) over a do-the-work sibling. \
     That is LABOR dressed as a DIRECTION question — a role violation: the user \
     decides direction, YOU do all the labor (global CLAUDE.md §division_of_labor). \
     DO NOT ask. DO the harder-correct thing THIS turn: run the rebuild / finish \
     the fix / complete the work, then verify. Only ask the user for a genuine \
     DIRECTION decision (which approach, what priority) or IRREVERSIBLE \
     authorization (push/delete/send) — never to choose whether to do work you can do."
        .to_owned()
}
#[cfg(test)]
#[path = "laziness_guard_test.rs"]
#[cfg(test)]
mod tests;
