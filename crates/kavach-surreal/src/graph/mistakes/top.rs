// Read side of the autonomous mistake loop: recurrence-ranked anti_pattern
// listing + practice-delta renderer. See decision/mistake-loop-close-read-graph.
use crate::error::Result;
use std::fmt::Write as _;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

/// One recurrence-ranked anti-pattern: the clustered behavioral lesson plus how
/// often a `mistake_event` has been routed to it (inbound `instance_of` edges).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AntiPatternRanked {
    /// Canonical node name, e.g. `anti.continuation_menu.395f9852`.
    pub name: String,
    /// Gate that fired the originating mistakes.
    pub gate: String,
    /// The do-instead rule reinjected to reinforce the fix (anti-parrot framing).
    pub correct_action: String,
    /// Recurrence count = inbound `instance_of` edges (the K-PRI signal).
    pub hit_count: i64,
}

/// Top-N anti-patterns ranked by recurrence (descending), then by name for a
/// stable tie-break.
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn top_anti_patterns(db: &Surreal<Db>, limit: usize) -> Result<Vec<AntiPatternRanked>> {
    // gate + correct_action are non-optional: upsert_anti_pattern always sets
    // both. Deserializing as String (not Option) fails closed on a malformed
    // node rather than silently defaulting to "" — the caller then falls back to
    // the legacy ledger instead of reinjecting a blank rule.
    #[derive(SurrealValue)]
    struct Row {
        name: String,
        gate: String,
        correct_action: String,
        hit_count: i64,
    }

    let q = "SELECT name, \
             properties.gate AS gate, \
             properties.correct_action AS correct_action, \
             count(<-instance_of<-entity) AS hit_count \
             FROM entity WHERE entity_type = 'anti_pattern'";
    let mut resp = db.query(q).await?;
    // A brand-new graph has never created the `entity` table (no migration / no
    // prior write), so SELECT raises "table does not exist". That is the empty
    // case — zero anti_patterns — not a failure: return [] so callers render
    // "no mistakes yet" instead of an error.
    let mut rows: Vec<Row> = match resp.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    rows.sort_by(|a, b| {
        b.hit_count
            .cmp(&a.hit_count)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows.truncate(limit);
    Ok(rows
        .into_iter()
        .map(|r| AntiPatternRanked {
            name: r.name,
            gate: r.gate,
            correct_action: r.correct_action,
            hit_count: r.hit_count,
        })
        .collect())
}

/// Render ranked anti-patterns as a `[PRACTICE_DELTA]` Mermaid contrast.
///
/// Each worst-practice (by recurrence) sits left, its known `correct_action` fix
/// right, joined by a `-.fixed by.->` edge. `None` when there are no anti-patterns.
/// SOURCE: roadmap.unit.mermaid-decision-architecture.
#[must_use]
pub fn practice_delta_mermaid(ranked: &[AntiPatternRanked]) -> Option<String> {
    if ranked.is_empty() {
        return None;
    }
    let mut out = String::from("graph LR\n  subgraph WORST[\"ledger hits (this codebase)\"]\n");
    for (i, ap) in ranked.iter().enumerate() {
        writeln!(
            out,
            "    w{i}[\"{}<br/>×{} via {}\"]",
            pd_escape(short_name(&ap.name)),
            ap.hit_count,
            pd_escape(&ap.gate)
        )
        .ok();
    }
    out.push_str("  end\n  subgraph BEST[\"do-instead (research-backed)\"]\n");
    for (i, ap) in ranked.iter().enumerate() {
        writeln!(out, "    b{i}[\"{}\"]", pd_escape(&ap.correct_action)).ok();
    }
    out.push_str("  end\n");
    for i in 0..ranked.len() {
        writeln!(out, "  w{i} -.fixed by.-> b{i}").ok();
    }
    Some(out)
}

/// Keep anti-patterns whose text shares a token with `focus`.
///
/// Searchable text = `name` slug + `gate` + `correct_action`; token = an
/// ASCII-lowercased alphanumeric run of ≥3 chars (drops noise like "a"/"to").
/// Empty `focus` (or focus with no usable tokens) passes everything through —
/// the whole-spine fallback for session-start, mirroring `decision_mermaid`.
#[must_use]
pub fn practice_delta_focus_filter(
    ranked: Vec<AntiPatternRanked>,
    focus: &[String],
) -> Vec<AntiPatternRanked> {
    let needles: Vec<String> = focus.iter().flat_map(|f| tokens(f)).collect();
    if needles.is_empty() {
        return ranked;
    }
    ranked
        .into_iter()
        .filter(|ap| {
            let hay: Vec<String> = tokens(short_name(&ap.name))
                .chain(tokens(&ap.gate))
                .chain(tokens(&ap.correct_action))
                .collect();
            needles.iter().any(|n| hay.iter().any(|h| h == n))
        })
        .collect()
}

/// ASCII-lowercased alphanumeric tokens of ≥3 chars from `s`.
fn tokens(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(str::to_ascii_lowercase)
}

/// Trim the canonical `anti.<slug>.<hash>` name to its readable slug.
fn short_name(name: &str) -> &str {
    name.strip_prefix("anti.")
        .and_then(|s| s.rsplit_once('.').map(|(slug, _)| slug))
        .unwrap_or(name)
}

/// Escape a label for a Mermaid quoted string.
fn pd_escape(label: &str) -> String {
    label.replace('"', "&quot;").replace(['\n', '\r'], " ")
}

/// Render a SINGLE mistake observation as a Mermaid DAG (banned behavior
/// `-.fixed by.->` correct action), in the same idiom as `practice_delta_mermaid`.
///
/// This is the stored `content` shape for a ledger row (#1699): a row's body is
/// a graph, not prose, so reinjection surfaces a structured banned→fix edge the
/// model parses directly. The `gate`/`turn`/`hit_count` ride on the banned node
/// label as recurrence context. SOURCE: roadmap.unit.mermaid-decision-architecture.
#[must_use]
pub fn mistake_row_mermaid(gate: &str, banned: &str, fix: &str, hit_count: u32) -> String {
    let mut out = String::from("graph LR\n");
    writeln!(
        out,
        "  w[\"BANNED [{}]<br/>{}<br/>×{hit_count}\"]:::banned",
        pd_escape(gate),
        pd_escape(banned)
    )
    .ok();
    writeln!(out, "  b[\"INSTEAD: {}\"]:::fix", pd_escape(fix)).ok();
    out.push_str("  w -.fixed by.-> b\n");
    out.push_str("  classDef banned fill:#9a3412,color:#fff;\n");
    out.push_str("  classDef fix fill:#1a7f37,color:#fff;\n");
    out
}

#[cfg(test)]
#[path = "top_test.rs"]
mod top_test;
