//! Gate-shaped card detection + ACT-driven imperative.
//!
//! A `GATE:`/`CLASS-B`-titled card is runnable work, but its title invites a
//! passive "Holding" hand-back that loops forever. On dispatch: STRIP the gate
//! words and replace the procedure with a DB-fetched imperative
//! (`gate.owner-gate-act-imperative`) telling the agent to DECOMPOSE the
//! buildable sub-task or DELETE the anchor — both completed this turn.
use crate::gates::directive_cache::dyn_directive;
/// Title-prefix / inline markers (matched case-insensitively) that flag a card
/// whose title declares an external/owner gate. These are the exact shapes the
/// looping transcript showed dispatched as "runnable". Kept as a small const
/// list, not a regex (engine has no regex dep on this path).
const GATE_MARKERS: [&str; 6] = [
    "gate:",
    "owner-gate:",
    "owner gate",
    "class-b",
    "await greenlight",
    "owner-action",
];
/// `true` iff `title` is gate-shaped — it announces an owner/external gate that
/// would otherwise be dispatched as runnable and looped on with "Holding".
#[must_use]
pub(super) fn is_gate_shaped(title: &str) -> bool {
    let lower = title.to_lowercase();
    GATE_MARKERS.iter().any(|m| lower.contains(m))
}
/// Strip the gate words from a gate-shaped title so the agent sees the WORK, not
/// the gate framing. Removes a leading `GATE:` / `OWNER-GATE:` prefix and any
/// inline gate marker token, collapsing surrounding separators. Falls back to the
/// original title if stripping would empty it (never hand back a blank card).
#[must_use]
pub(super) fn strip_gate_words(title: &str) -> String {
    let mut out = title.to_owned();
    // Remove longest markers first: "gate:" is a substring of "owner-gate:", so
    // stripping the short one first would mangle the longer (left "OWNER-").
    let mut markers = GATE_MARKERS;
    markers.sort_unstable_by_key(|m| core::cmp::Reverse(m.len()));
    // Case-insensitive removal of each marker. We rebuild via lowercase-find so
    // the original casing of the SURVIVING text is preserved.
    for marker in markers {
        loop {
            let lower = out.to_lowercase();
            let Some(idx) = lower.find(marker) else { break };
            let end = idx.saturating_add(marker.len());
            // Splice out [idx, end). `idx`/`end` are byte offsets from a
            // lowercase copy of the SAME bytes (ASCII markers), so they index
            // `out` safely without splitting a char.
            let mut rebuilt = String::with_capacity(out.len());
            if let Some(head) = out.get(..idx) {
                rebuilt.push_str(head);
            }
            if let Some(tail) = out.get(end..) {
                rebuilt.push_str(tail);
            }
            out = rebuilt;
        }
    }
    // Collapse separators left behind (leading ":", "-", whitespace runs).
    let cleaned = out
        .trim_matches(|c: char| c.is_whitespace() || c == ':' || c == '-' || c == '—')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.is_empty() {
        title.to_owned()
    } else {
        cleaned
    }
}
/// The ACT-driven imperative injected when a gate-shaped card is dispatched.
///
/// Imperative register only: the agent extracts the buildable sub-task or
/// deletes the un-buildable anchor THIS turn — it never "Holds". The remedy
/// body is research-/DB-refreshed via `directive_cache`
/// (`gate.owner-gate-act-imperative`), fail-soft to the proven literal below.
/// `stripped` is the gate-words-removed title so the directive names the WORK.
#[must_use]
pub(super) fn act_imperative(stripped: &str) -> String {
    let body = dyn_directive(
        "gate.owner-gate-act-imperative",
        "This card's title carried owner/external-gate words; they have been \
         STRIPPED — what remains is the WORK. 'Holding', 'Owner — please …', \
         'CLASS-B owner gate', and any hand-back are FORBIDDEN: you hold the \
         shell, so the agent action is yours.\n\
         DO THIS TURN (do not narrate, do not hold, do not re-dispatch this same \
         card):\n\
         1. SPLIT: if a buildable sub-task exists inside this card (code/config/test \
         work an agent CAN do without owner credentials or provisioning), carve it \
         into its own runnable card (`kavach db write --category roadmap`), claim it, \
         and BUILD it now.\n\
         2. DELETE: if the ENTIRE remaining card is un-buildable by an agent (it needs \
         only an owner deploy/greenlight/provision and no code is left to write), \
         DELETE the anchor — `kavach db delete --category roadmap --key <key>` — per \
         §delete_not_park; DELETE it so it never re-dispatches.\n\
         3. Then RESUME dispatch from the reconciled kanban.\n\
         Do NOT re-emit this card unchanged and do NOT answer 'Holding' — either \
         re-creates the 300-iteration loop this directive exists to kill. KEEP WORKING; \
         only the user's `Esc` stops the loop.",
    );
    format!(
        "[GATE_STRIPPED — ACT, DO NOT HOLD] The dispatched card was gate-shaped; \
         its gate words are removed. WORK: \"{stripped}\". {body}"
    )
}
#[cfg(test)]
#[path = "gate_strip_test.rs"]
mod tests;
