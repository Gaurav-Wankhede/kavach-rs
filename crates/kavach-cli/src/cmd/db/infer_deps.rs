// `kavach db infer-deps` — derive `depends_on` edges from card-key naming.
//
// The kanban DAG only segregates into tiers when cards declare prerequisites.
// Most historical cards were authored as dotted namespaces with a trailing
// sequence token (`unit.harness-rl.p7`, `unit.loop-eng-injection.f4`) but never
// got an explicit `--depends-on`, so the tier view collapsed every card into
// TIER 0. This command infers the obvious intra-namespace ordering — a card
// whose trailing token is sequence N depends on the same-namespace sibling whose
// matching token is sequence N-1 — and (only with --apply) writes those edges
// through the daemon's normal write path (`update_key` + `depends_on`), which is
// the same projection the GUI tier layout reads.
//
// HEURISTIC, NOT TRUTH: it is dry-run by default and prints every proposed edge
// for human review. --apply is the explicit opt-in to mutate real roadmap rows.
use crate::cmd::db::rpc_client::{self, WriteRequest};
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
/// A card reduced to the two facts inference needs: its full key and the parsed
/// `(namespace, token_letter, sequence)` of its trailing segment (if any).
struct Card {
    key: String,
    title: String,
    content: String,
    seq: Option<Seq>,
}
/// The parsed trailing sequence of a card key: the namespace it shares with its
/// siblings, the alphabetic token kind (`p`/`f`/`step`/…, empty for a bare
/// number), and the numeric position. Two cards are sequential siblings iff they
/// share `namespace` AND `token` and differ by 1 in `n`.
#[derive(PartialEq, Eq)]
struct Seq {
    namespace: String,
    token: String,
    n: u64,
}
/// One inferred edge: `card` should declare `depends_on prereq`.
struct InferredEdge {
    card: String,
    prereq: String,
}
/// Recognised alphabetic sequence-token prefixes, longest-first so `phase`
/// matches before a bare `p`. A trailing segment of the form `<token><digits>`
/// (e.g. `p7`, `phase2`, `step10`) or bare `<digits>` is a sequence position.
const TOKENS: [&str; 8] = ["phase", "wave", "task", "iter", "step", "p", "f", "v"];
/// Parse a card key's trailing sequence segment.
///
/// The namespace is everything up to (and excluding) the final `.`-delimited
/// segment; the final segment is matched against `<token><number>`. A key with
/// no dot, or whose final segment is not `<token><number>`, yields `None` (it is
/// a singleton and never an inferred dependant).
fn parse_seq(key: &str) -> Option<Seq> {
    let dot = key.rfind('.')?;
    let (ns, last) = key.split_at(dot);
    let last = last.strip_prefix('.').unwrap_or(last);
    // Sub-tokenise on `-` too: `p7-epsilon-greedy` -> the leading `p7` carries
    // the sequence, the trailing words are a human label.
    let head = last.split('-').next().unwrap_or(last);
    for tok in TOKENS {
        if let Some(digits) = head.strip_prefix(tok)
            && !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
        {
            let n = digits.parse::<u64>().ok()?;
            return Some(Seq {
                namespace: ns.to_owned(),
                token: tok.to_owned(),
                n,
            });
        }
    }
    // Bare trailing number (`unit.foo.3`): token is empty.
    if !head.is_empty() && head.bytes().all(|b| b.is_ascii_digit()) {
        let n = head.parse::<u64>().ok()?;
        return Some(Seq {
            namespace: ns.to_owned(),
            token: String::new(),
            n,
        });
    }
    None
}
/// Pure inference core: given the card keys, return the edges that the
/// sequence-token heuristic proposes, deduplicated and deterministic.
///
/// An edge `card -> prereq` is proposed when `card` has sequence N and a
/// DISTINCT card in the SAME namespace with the SAME token has sequence N-1.
/// The N-1 predecessor must EXIST as a real card (no edges to phantom keys), and
/// is chosen as the unique such sibling; if two cards collide on the same
/// `(namespace, token, n)` the predecessor is ambiguous and the edge is skipped
/// (fail-safe: never guess between duplicates).
fn infer(cards: &[Card]) -> Vec<InferredEdge> {
    let mut edges = Vec::new();
    for c in cards {
        let Some(seq) = &c.seq else { continue };
        let Some(prev_n) = seq.n.checked_sub(1) else {
            continue;
        };
        let mut matches = cards.iter().filter(|o| {
            o.key != c.key
                && o.seq.as_ref().is_some_and(|s| {
                    s.namespace == seq.namespace && s.token == seq.token && s.n == prev_n
                })
        });
        let (Some(prereq), None) = (matches.next(), matches.next()) else {
            continue; // zero predecessors, or ambiguous (>1) -> skip
        };
        edges.push(InferredEdge {
            card: c.key.clone(),
            prereq: prereq.key.clone(),
        });
    }
    edges
}
/// Whether `content` already declares `prereq` on a `DEPENDS_ON:` line. Mirrors
/// the GUI's `kanban::deps::declared_deps` parse (comma/space-separated keys
/// after a `DEPENDS_ON:` prefix) so the append below is idempotent against the
/// EXACT convention the tier layout reads — re-running never duplicates a dep.
fn already_declares(content: &str, prereq: &str) -> bool {
    content.lines().any(|raw| {
        raw.trim().strip_prefix("DEPENDS_ON:").is_some_and(|rest| {
            rest.split([',', ' ', '\t'])
                .map(str::trim)
                .any(|k| k == prereq)
        })
    })
}
/// Append a `DEPENDS_ON: <prereq>` line to `content`, returning the new content.
/// The tier GUI (`kanban::deps::declared_deps`) parses this line from card TEXT
/// — the relationship-graph edge from `--depends-on` alone is NOT what the tier
/// layout reads, so the dep MUST land in content to segregate the tiers.
fn append_dep_line(content: &str, prereq: &str) -> String {
    let sep = if content.is_empty() || content.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{content}{sep}DEPENDS_ON: {prereq}\n")
}
/// `kavach db infer-deps` entry point. Lists roadmap cards, infers edges, prints
/// the proposal, and (only with `apply`) writes each edge via the daemon.
pub(crate) fn run(project: &str, apply: bool) -> i32 {
    let entries = match rpc_client::query(project, Some("roadmap"), true) {
        Ok(r) => r.entries,
        Err(e) => return emit(&format!("infer-deps: list cards: {e}")),
    };
    let cards: Vec<Card> = entries
        .into_iter()
        .map(|e| Card {
            seq: parse_seq(&e.key),
            key: e.key,
            title: e.title,
            content: e.content.unwrap_or_default(),
        })
        .collect();
    let title_of = |k: &str| {
        cards
            .iter()
            .find(|c| c.key == k)
            .map_or("", |c| c.title.as_str())
            .to_owned()
    };
    let edges = infer(&cards);
    if edges.is_empty() {
        return emit("infer-deps: no sequence-based dependencies inferred.");
    }
    let header = if apply {
        format!("infer-deps: applying {} edge(s):", edges.len())
    } else {
        format!(
            "infer-deps: {} edge(s) proposed (DRY RUN — re-run with --apply to write):",
            edges.len()
        )
    };
    let lines: String = edges
        .iter()
        .map(|e| {
            format!(
                "\n  {}\n      depends_on -> {}  ({})",
                e.card,
                e.prereq,
                title_of(&e.prereq)
            )
        })
        .collect::<Vec<String>>()
        .concat();
    if let Err(io_err) = print_or_exit(&format!("{header}{lines}")) {
        return into_exit_code(io_err);
    }
    if !apply {
        return 0;
    }
    let card_by_key = |k: &str| cards.iter().find(|c| c.key == k);
    let mut failures = 0_u32;
    let mut skipped = 0_u32;
    for e in &edges {
        let Some(card) = card_by_key(&e.card) else {
            // The card vanished between list and write (concurrent delete);
            // skip rather than write a phantom row.
            continue;
        };
        if already_declares(&card.content, &e.prereq) {
            skipped = skipped.saturating_add(1);
            continue; // idempotent: already declared, nothing to do
        }
        // Append to CONTENT (what the tier GUI parses) AND pass --depends-on so
        // the relationship-graph projection is also created — the two stores stay
        // consistent. update_key keeps the existing row in place.
        let new_content = append_dep_line(&card.content, &e.prereq);
        let deps = [e.prereq.clone()];
        let req = WriteRequest {
            project,
            category: "roadmap",
            key: &e.card,
            title: &title_of(&e.card),
            content: Some(&new_content),
            new: false,
            update_key: Some(&e.card),
            priority: None,
            exec_prompt: None,
            depends_on: &deps,
        };
        if let Err(err) = rpc_client::write(&req) {
            failures = failures.saturating_add(1);
            // Best-effort progress line; the failure count is the authoritative
            // signal returned below, so a stdout hiccup here is non-fatal.
            print_or_exit(&format!("  FAILED {}: {err}", e.card)).ok();
        }
    }
    if skipped > 0 {
        print_or_exit(&format!(
            "infer-deps: {skipped} edge(s) already declared, skipped."
        ))
        .ok();
    }
    if failures == 0 {
        if let Err(io_err) =
            print_or_exit("infer-deps: all edges written. Re-deploy to refresh the tier GUI.")
        {
            return into_exit_code(io_err);
        }
        0
    } else {
        emit(&format!("infer-deps: {failures} edge(s) failed to write"))
    }
}
fn emit(msg: &str) -> i32 {
    match print_or_exit(msg) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}
#[cfg(test)]
#[path = "infer_deps_test.rs"]
mod tests;
