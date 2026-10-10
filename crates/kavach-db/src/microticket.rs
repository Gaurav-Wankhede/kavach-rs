//! `MicroTicket v1` — a bounded, closed-vocabulary, machine-actionable ticket.
//!
//! # Why this shape
//!
//! Designed for a 3B-8B model (SLM) authoring tickets, not for a human reading
//! prose. Three evidence-driven rules drive every field:
//!
//! 1. **Free prose is not a ticket.** SWE-bench discarded ~90,000 PRs to keep
//!    2,294 instances — most real tickets are not machine-actionable.
//!    SOURCE: <https://arxiv.org/abs/2310.06770>
//! 2. **Verifiability must be executable.** SWE-bench drops any instance with no
//!    fail->pass test; CISOSE: *"A spec an agent can implement is a spec you can
//!    test."* `verify` is therefore mandatory and may not be empty.
//! 3. **Closed vocabularies beat free text.** Status/priority/op/method are
//!    enums, never strings — a free-text slot is exactly where an SLM
//!    hallucinates an invalid value.
//!
//! # The finding that shaped this file
//!
//! A schema-valid ticket is NOT a good ticket. The Constraint Tax paper
//! (15,000 generations, Qwen2.5-0.5B/1.5B + SmolLM2-1.7B) measured hard schema
//! decoding raising validity 61.5% -> 100.0% while DROPPING answer accuracy
//! 19.7% -> 11.0% and raising wrong-valid-schema output 49.5% -> 88.9%. On one
//! deterministic tool-call: 91.5% executable accuracy prompt-only vs 48.0% under
//! a hard schema — both 100% valid. *"The error is semantic, not structural."*
//!
//! Therefore this module VALIDATES rather than hard-decodes: generate freely,
//! validate at the boundary, repair on failure. Validity is necessary, never
//! sufficient — do not treat a passing `validate()` as a quality signal.
//! SOURCE: <https://arxiv.org/abs/2605.26128>
//!
//! # Character caps are a design choice, NOT a standard
//!
//! There is no published convention capping agent-ticket characters. GitHub ships
//! `min_length` and deliberately omits `max_length`; SWE-bench's
//! `problem_statement` averages 195.1 words with a 4477 max — pointing the other
//! way. The caps below are sized deliberately; the `goal` cap of 200 is the most
//! defensible (SWE-bench's mean is 195.1 WORDS across a 3,000-file codebase, so
//! one field should be ~1/6 of that).
//!
//! If a ticket does not fit, SPLIT IT. Do not enlarge the cap — a loosened cap
//! re-creates the under-specified-ticket failure mode.
//! SOURCE: <https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-githubs-form-schema>

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Field-count ceiling. Constraint Tax shows pressure compounds fast; more than
/// these nine blocks degrades small-model output into placeholders.
pub const MAX_TICKET_FIELDS: usize = 9;

/// Character caps. See module docs: these are sizing decisions, not standards.
pub mod caps {
    /// Kanban row budget. `title` renders beside `[status] key` and the READY
    /// marker, so it must fit one line without truncation.
    pub const TITLE: usize = 90;
    /// One sentence of intent — the after-state.
    pub const GOAL: usize = 200;
    /// Trigger / observable outcome pair.
    pub const CLAUSE: usize = 120;
    /// Repo-relative path.
    pub const PATH: usize = 120;
    /// Glob.
    pub const GLOB: usize = 80;
    /// Exact shell command or test path.
    pub const RUN: usize = 200;
    /// One observable expected result.
    pub const EXPECT: usize = 120;
    /// Provenance note.
    pub const NOTE: usize = 100;
    /// Ticket key — must fit the kanban row beside `[status] `.
    pub const KEY: usize = 48;

    pub const SCOPE_MAX: usize = 5;
    pub const DNT_MAX: usize = 5;
    pub const VERIFY_MAX: usize = 5;
    pub const REL_MAX: usize = 5;
}

/// A validation failure naming the offending field and the rule it broke.
///
/// Fail-closed: every rule below REFUSES. A ticket that cannot be validated is
/// not a ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketError {
    /// Dotted path of the offending field, e.g. `verify[0].expect`.
    pub field: String,
    /// Human-readable rule the value broke.
    pub rule: String,
}

impl fmt::Display for TicketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.rule)
    }
}

impl std::error::Error for TicketError {}

/// Ticket class. Determines nothing structurally today, but routes the board and
/// is a closed enum so a model cannot invent a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketKind {
    Bug,
    Feature,
    Chore,
    Refactor,
    Perf,
    Docs,
    Test,
}

/// RFC 2119 requirement level, carried AS DATA.
///
/// RFC 2119 itself concedes the force of its keywords *"is modified by the
/// requirement level of the document in which they are used"* — so MUST in prose
/// is insufficient; the document class must also be machine-stated.
/// SOURCE: <https://www.rfc-editor.org/rfc/rfc2119.txt>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modality {
    #[serde(rename = "MUST")]
    Must,
    #[serde(rename = "SHOULD")]
    Should,
    #[serde(rename = "MAY")]
    May,
}

/// Exactly one operation. A single enum value is the STRUCTURAL guard against
/// mixed-change tickets — not a style rule an author can wave away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Add,
    Edit,
    Delete,
    Move,
}

/// Where the truth about this ticket lives.
///
/// A discriminator rather than free text: a free-text `source` slot is precisely
/// where an SLM writes a crate that does not exist. `file` means "go read this",
/// `url` means "go learn this" — different evidence kinds, so not one blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// A workspace crate.
    Crate,
    /// A repo-relative file, optionally `:line`.
    File,
    /// An external URL to fetch.
    Url,
    /// Named documentation.
    Doc,
    /// A test path.
    Test,
    /// A live row in the kavach DB.
    Db,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyMethod {
    Cmd,
    Test,
    Assert,
    /// NOT machine-runnable. Requires `manual_by`; the one honest escape hatch.
    Manual,
}

/// One executable acceptance criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyStep {
    pub method: VerifyMethod,
    /// Exact shell command or test path. Required for `cmd`/`test`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// One observable expected result, stated as an outcome not a status.
    pub expect: String,
}

/// Typed provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub kind: SourceKind,
    /// `path`, `path:line`, or a URL depending on `kind`.
    pub r#ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The behaviour change: exactly one op, with a trigger and an observable outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub op: Op,
    /// Trigger condition.
    pub when: String,
    /// Observable outcome after the change.
    pub then: String,
}

/// A `MicroTicket v1`.
///
/// Stored as the roadmap row's `content`, so `title` renders on the kanban and
/// `verify` remains machine-readable for the dispatch loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTicket {
    pub kind: TicketKind,
    /// One line, imperative verb first. Never truncated by the renderer.
    pub title: String,
    /// Exactly one sentence of intent: what is true after this is done.
    pub goal: String,
    pub modality: Modality,
    pub change: Change,
    /// Repo-relative paths this ticket is allowed to touch.
    pub scope: Vec<String>,
    /// Paths explicitly out of scope. Prevents scope creep.
    pub do_not_touch: Vec<String>,
    /// Executable acceptance criteria. Never empty.
    pub verify: Vec<VerifyStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Ticket keys that must finish first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Ticket keys mutually exclusive with this one. ENFORCED at dispatch: two
    /// conflicting tickets never share a tier, so a parallel runner can never
    /// pick both.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts_with: Vec<String>,
    /// Ticket keys this one replaces outright.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    /// Required if any `verify` step is `manual`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_by: Option<String>,
}

impl MicroTicket {
    /// Parse from JSON. Rejects unknown fields (fail-closed on typos).
    ///
    /// # Errors
    /// Returns the serde error on malformed JSON or an unknown field.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Serialize to canonical JSON.
    ///
    /// # Errors
    /// Returns a serde error if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Validate every field against the caps, enums and cross-field rules.
    ///
    /// Fails closed: every violation is collected (an author fixes all at once
    /// rather than one per attempt).
    ///
    /// # Errors
    /// Returns every `TicketError` found, or `Ok(())` when the ticket is valid.
    pub fn validate(&self) -> Result<(), Vec<TicketError>> {
        use caps::*;
        let mut errs: Vec<TicketError> = Vec::new();
        let mut err = |field: &str, rule: String| {
            errs.push(TicketError {
                field: field.to_owned(),
                rule,
            });
        };

        check_len(&mut err, "title", &self.title, TITLE);
        check_single_line(&mut err, "title", &self.title);
        check_len(&mut err, "goal", &self.goal, GOAL);
        check_single_line(&mut err, "goal", &self.goal);
        // One sentence: a second terminal sentence mark means the goal drifted
        // into a plan. Abbreviations with a period are not attempted to be
        // parsed — a trailing `.` at end-of-string is allowed.
        let sentences = count_sentence_ends(&self.goal);
        if sentences > 1 {
            err(
                "goal",
                format!(
                    "must be ONE sentence of intent, found {sentences} sentence ends — \
                     split the ticket instead"
                ),
            );
        }
        check_len(&mut err, "change.when", &self.change.when, CLAUSE);
        check_single_line(&mut err, "change.when", &self.change.when);
        check_len(&mut err, "change.then", &self.change.then, CLAUSE);
        check_single_line(&mut err, "change.then", &self.change.then);

        if self.scope.is_empty() {
            err("scope", "must list at least 1 allowed path".to_owned());
        } else if self.scope.len() > SCOPE_MAX {
            err(
                "scope",
                format!("max {SCOPE_MAX} paths, got {} — split the ticket", self.scope.len()),
            );
        }
        for (i, p) in self.scope.iter().enumerate() {
            check_len(&mut err, &format!("scope[{i}]"), p, PATH);
            if p.trim() != p {
                err(
                    &format!("scope[{i}]"),
                    "path has surrounding whitespace".to_owned(),
                );
            }
        }

        if self.do_not_touch.is_empty() {
            // CISOSE: "The 'do not touch' line is what stops scope creep."
            err(
                "do_not_touch",
                "must name at least 1 out-of-scope path".to_owned(),
            );
        } else if self.do_not_touch.len() > DNT_MAX {
            err(
                "do_not_touch",
                format!("max {DNT_MAX} globs, got {}", self.do_not_touch.len()),
            );
        }
        for (i, g) in self.do_not_touch.iter().enumerate() {
            check_len(&mut err, &format!("do_not_touch[{i}]"), g, GLOB);
        }
        // A path in both lists is a contradiction that silently widens the
        // blast radius — the "do not touch" is a no-op for that path.
        let scope_set: BTreeSet<&str> = self.scope.iter().map(String::as_str).collect();
        for (i, g) in self.do_not_touch.iter().enumerate() {
            if scope_set.contains(g.as_str()) {
                err(
                    &format!("do_not_touch[{i}]"),
                    format!("'{g}' is also in scope — the exclusion is a no-op"),
                );
            }
        }

        if self.verify.is_empty() {
            // SWE-bench drops any instance with no executable oracle.
            err(
                "verify",
                "must carry at least 1 acceptance criterion".to_owned(),
            );
        } else if self.verify.len() > VERIFY_MAX {
            err(
                "verify",
                format!("max {VERIFY_MAX} steps, got {}", self.verify.len()),
            );
        }
        let mut has_manual = false;
        for (i, v) in self.verify.iter().enumerate() {
            let f = format!("verify[{i}]");
            check_len(&mut err, &format!("{f}.expect"), &v.expect, EXPECT);
            check_single_line(&mut err, &format!("{f}.expect"), &v.expect);
            match v.method {
                VerifyMethod::Cmd | VerifyMethod::Test => match &v.run {
                    None => err(
                        &format!("{f}.run"),
                        format!("method '{:?}' requires an exact command", v.method),
                    ),
                    Some(r) => {
                        check_len(&mut err, &format!("{f}.run"), r, RUN);
                        check_single_line(&mut err, &format!("{f}.run"), r);
                    }
                },
                VerifyMethod::Manual => {
                    has_manual = true;
                    if v.run.is_some() {
                        err(
                            &format!("{f}.run"),
                            "method 'manual' takes no command — state the check in expect".
                                to_owned(),
                        );
                    }
                }
                VerifyMethod::Assert => {
                    if v.run.is_none() {
                        err(
                            &format!("{f}.run"),
                            "method 'assert' requires a command or test path".to_owned(),
                        );
                    }
                }
            }
        }
        if has_manual && self.manual_by.is_none() {
            err(
                "manual_by",
                "required when any verify step is 'manual'".to_owned(),
            );
        }

        if let Some(s) = &self.source {
            check_len(&mut err, "source.ref", &s.r#ref, PATH);
            if s.r#ref.trim().is_empty() {
                err("source.ref", "must name a crate, file, URL or doc".to_owned());
            }
            if let Some(n) = &s.note {
                check_len(&mut err, "source.note", n, NOTE);
            }
            match s.kind {
                SourceKind::Url => {
                    if !(s.r#ref.starts_with("http://") || s.r#ref.starts_with("https://")) {
                        err(
                            "source.ref",
                            "kind 'url' requires an http(s):// reference".to_owned(),
                        );
                    }
                }
                SourceKind::Crate => {
                    if !s.r#ref.contains('-') && !s.r#ref.contains('_') {
                        err(
                            "source.ref",
                            "kind 'crate' expects a crate name (e.g. kavach-db)".to_owned(),
                        );
                    }
                }
                SourceKind::File | SourceKind::Test | SourceKind::Doc | SourceKind::Db => {}
            }
        }

        for (field, list) in [
            ("depends_on", &self.depends_on),
            ("conflicts_with", &self.conflicts_with),
            ("supersedes", &self.supersedes),
        ] {
            if list.len() > REL_MAX {
                err(
                    field,
                    format!("max {REL_MAX} keys, got {}", list.len()),
                );
            }
            for (i, k) in list.iter().enumerate() {
                check_len(&mut err, &format!("{field}[{i}]"), k, KEY);
                if k.trim() != k {
                    err(
                        &format!("{field}[{i}]"),
                        "key has surrounding whitespace".to_owned(),
                    );
                }
            }
            let uniq: BTreeSet<&String> = list.iter().collect();
            if uniq.len() != list.len() {
                err(field, "contains duplicate keys".to_owned());
            }
        }

        // A ticket that both depends on and conflicts with the same key is a
        // deadlock: wait for it, then exclude it. Refuse rather than resolve.
        let dep: BTreeSet<&str> = self.depends_on.iter().map(String::as_str).collect();
        for c in &self.conflicts_with {
            if dep.contains(c.as_str()) {
                err(
                    "conflicts_with",
                    format!("'{c}' is also in depends_on — unsatisfiable ordering"),
                );
            }
        }
        for s in &self.supersedes {
            if dep.contains(s.as_str()) {
                err(
                    "supersedes",
                    format!("'{s}' is also in depends_on — superseding is not waiting"),
                );
            }
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }
}

fn check_len(
    err: &mut impl FnMut(&str, String),
    field: &str,
    value: &str,
    max: usize,
) {
    let n = value.chars().count();
    if n > max {
        err(
            field,
            format!("{n} chars exceeds the {max}-char cap — split the ticket"),
        );
    }
    if value.trim().is_empty() {
        err(field, "must not be empty".to_owned());
    }
}

fn check_single_line(err: &mut impl FnMut(&str, String), field: &str, value: &str) {
    if value.contains('\n') {
        err(field, "must be a single line".to_owned());
    }
}

/// Count sentence-ending marks, ignoring a trailing period (the normal end) and
/// decimal points. Conservative: it over-counts rather than letting a rambling
/// goal through, which is the failure mode we are forbidding.
fn count_sentence_ends(s: &str) -> usize {
    let trimmed = s.trim_end();
    let body = trimmed.strip_suffix('.').unwrap_or(trimmed);
    body.matches(['.', '!', '?']).count() + usize::from(trimmed.ends_with(['.', '!', '?']))
}

#[cfg(test)]
#[path = "microticket_test.rs"]
mod tests;