//! `kavach schema --vendor <tag>` / `--all` — print a vendor's LIVE upstream
//! hook-contract schema source so an operator or agent can fetch + diff the
//! CURRENT contract instead of trusting a frozen in-binary assumption. The URLs
//! are the canonical [`SchemaSource`] on the [`Vendor`] enum (single source of
//! truth — `kavach_hook`). SOURCE: roadmap universal.vendor-schema-urls.

use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use kavach_hook::{SchemaSource, Vendor};

/// One human-readable report line for a vendor's schema source: kind + URL, plus
/// the companion prose page when the primary source is a machine-readable schema.
/// `SchemaSource` is `#[non_exhaustive]` (foreign crate), so the wildcard arm is
/// mandatory; it fails CLOSED — an unknown future kind is reported as opaque
/// rather than silently dropped.
fn report_line(v: Vendor) -> String {
    let name = v.name();
    match v.schema_url() {
        SchemaSource::JsonSchema { url, prose } => {
            format!("[{name}] json-schema (fetchable): {url}\n         prose: {prose}")
        }
        SchemaSource::Prose { url } => {
            format!("[{name}] prose-only (no JSON Schema published): {url}")
        }
        other => format!("[{name}] unknown schema source kind: {}", other.url()),
    }
}

/// `kavach schema` entry. `--all` (or no `--vendor`) lists every vendor; a single
/// `--vendor` tag prints one. An unknown tag is rejected (fail-closed).
pub(crate) fn run(vendor: Option<&str>, all: bool) -> i32 {
    let targets = match resolve(vendor, all) {
        Ok(t) => t,
        Err(msg) => return report_err(&format!("kavach schema: {msg}")),
    };
    for v in targets {
        if let Err(io) = print_or_exit(&report_line(v)) {
            return into_exit_code(io);
        }
    }
    0
}

/// Resolve the vendor set: `--all` (or no tag) → every vendor; a known tag → one;
/// an unknown tag → error.
fn resolve(vendor: Option<&str>, all: bool) -> Result<Vec<Vendor>, String> {
    match vendor {
        _ if all => Ok(Vendor::all().to_vec()),
        None => Ok(Vendor::all().to_vec()),
        Some(tag) => Vendor::from_tag(tag).map(|v| vec![v]).ok_or_else(|| {
            format!("unknown --vendor '{tag}' (expected: cc|cursor|codex|antigravity|gemini|pi|kimi)")
        }),
    }
}

/// stderr report → exit 1 (collapses to the IO exit code if stderr itself fails).
fn report_err(msg: &str) -> i32 {
    crate::cmd::io_safe::ewrite_or_exit(msg).map_or_else(into_exit_code, |()| 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_vendor() {
        assert_eq!(resolve(None, true).unwrap().len(), Vendor::all().len());
        // No tag also defaults to listing all (the `--all` ergonomic shortcut).
        assert_eq!(resolve(None, false).unwrap().len(), Vendor::all().len());
    }

    #[test]
    fn known_tag_resolves_to_one() {
        assert_eq!(
            resolve(Some("cursor"), false).unwrap(),
            vec![Vendor::Cursor]
        );
        // `gemini` is an honored alias for Antigravity (the migration target).
        assert_eq!(
            resolve(Some("gemini"), false).unwrap(),
            vec![Vendor::Antigravity]
        );
    }

    #[test]
    fn unknown_tag_is_error() {
        assert!(resolve(Some("bogus"), false).is_err());
    }

    #[test]
    fn json_schema_vendors_report_fetchable() {
        // CC + Cursor publish a machine-readable schema; the line must say so.
        assert!(report_line(Vendor::ClaudeCode).contains("json-schema"));
        assert!(report_line(Vendor::Cursor).contains("json-schema"));
    }

    #[test]
    fn prose_only_vendors_report_prose() {
        // Codex + Antigravity publish no JSON Schema — must NOT claim fetchable.
        let codex = report_line(Vendor::Codex);
        assert!(codex.contains("prose-only"));
        assert!(!codex.contains("json-schema"));
        assert!(report_line(Vendor::Antigravity).contains("prose-only"));
    }
}
