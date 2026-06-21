//! `[STACK_FIT]` renderer: the chosen language/tech-stack bound to its boundaries.
//!
//! Read-side VIEW over `app_spec` rows keyed `stack.<component>` (title =
//! component, content = the invariant it imposes), rendered as a Mermaid
//! `graph TD`. AGNOSTIC: nothing is hardcoded — components and boundaries come
//! entirely from the project's own `app_spec`.
//! SOURCE: roadmap.unit.mermaid-decision-architecture.
use crate::error::Result;
use std::fmt::Write as _;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

/// One stack component and the non-negotiable boundary it imposes.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StackInvariant {
    /// Component name, e.g. "Dioxus 0.7.9" or "Axum 0.8".
    pub component: String,
    /// The non-negotiable boundary it imposes (free text from the `app_spec` row).
    pub invariant: String,
}

/// Row shape for the `stack.<component>` `app_spec` query.
#[derive(SurrealValue)]
struct StackRow {
    title: String,
    content: String,
}

/// Bare-id projection from the `project` lookup.
#[derive(SurrealValue)]
struct IdRow {
    id: surrealdb_types::RecordId,
}

/// Fetch the project's `stack.*` `app_spec` rows as ordered invariants.
///
/// # Errors
/// Propagates `Error::Surreal` on query failure. A project with no `stack.*`
/// rows (or no project record) yields an empty vec — the fail-soft "nothing to
/// inject" case, not an error.
pub async fn stack_invariants(db: &Surreal<Db>, project_slug: &str) -> Result<Vec<StackInvariant>> {
    let proj_q = "SELECT id FROM project WHERE slug = $slug LIMIT 1";
    let mut p_resp = db
        .query(proj_q)
        .bind(("slug", project_slug.to_owned()))
        .await?;
    let Some(IdRow { id: pid }): Option<IdRow> = p_resp.take(0)? else {
        return Ok(Vec::new());
    };

    // app_spec rows whose key marks them a stack component. string::starts_with
    // keeps the filter in the DB; ORDER BY entry_key gives stable render order.
    let row_q = "SELECT title, content FROM app_spec \
                 WHERE project = $pid AND string::starts_with(entry_key, 'stack.') \
                 ORDER BY entry_key ASC";
    let mut r = db.query(row_q).bind(("pid", pid)).await?;
    let rows: Vec<StackRow> = match r.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(rows
        .into_iter()
        .map(|row| StackInvariant {
            component: row.title,
            invariant: row.content,
        })
        .collect())
}

/// Render stack invariants as a `[STACK_FIT]` Mermaid `graph TD`: each component
/// node points at the boundary it imposes. `None` when there are no invariants
/// (nothing to bind).
#[must_use]
pub fn stack_fit_mermaid(invariants: &[StackInvariant]) -> Option<String> {
    if invariants.is_empty() {
        return None;
    }
    let mut out = String::from("graph TD\n");
    for (i, inv) in invariants.iter().enumerate() {
        writeln!(out, "  c{i}[\"{}\"] --> b{i}{{\"{}\"}}", sf_escape(&inv.component), sf_escape(&inv.invariant)).ok();
    }
    Some(out)
}

/// Escape a label for a Mermaid quoted string.
fn sf_escape(label: &str) -> String {
    label.replace('"', "&quot;").replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_component_to_boundary_edges() {
        let inv = vec![
            StackInvariant {
                component: "Dioxus 0.7.9".to_owned(),
                invariant: "web-sys Location gap — route through BFF".to_owned(),
            },
            StackInvariant {
                component: "Axum 0.8".to_owned(),
                invariant: "own origin; clients reach via same-origin proxy".to_owned(),
            },
        ];
        let m = stack_fit_mermaid(&inv).expect("non-empty");
        assert!(m.starts_with("graph TD\n"), "{m}");
        assert!(m.contains("Dioxus 0.7.9"), "{m}");
        assert!(m.contains("c0[") && m.contains("--> b0{"), "{m}");
        assert!(m.contains("Axum 0.8"), "{m}");
    }

    #[test]
    fn empty_yields_none() {
        assert!(stack_fit_mermaid(&[]).is_none());
    }

    #[test]
    fn escapes_quotes_in_labels() {
        let inv = vec![StackInvariant {
            component: "lib \"x\"".to_owned(),
            invariant: "no \"raw\" env".to_owned(),
        }];
        let m = stack_fit_mermaid(&inv).expect("non-empty");
        assert!(!m.contains("\"x\""), "raw quote must be escaped: {m}");
        assert!(m.contains("&quot;"), "{m}");
    }
}
