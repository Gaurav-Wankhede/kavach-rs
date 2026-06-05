//! Project-context resolution: load `app_spec`/decision/roadmap rows + tier for
//! the CWD's project via the (sync gate hot-path) blocking tokio runtime.
use crate::error::EngineError;

/// Resolved spec context for the active project.
pub(super) struct ProjectContext {
    pub(super) slug: String,
    pub(super) tier: kavach_types::ProjectTier,
    pub(super) rows: Vec<(String, String)>,
}

fn parse_tier_from_content(content: &str) -> Option<kavach_types::ProjectTier> {
    for line in content.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("tier=") {
            let value = rest.split([';', ' ', '\n']).next().unwrap_or("").trim();
            return kavach_types::ProjectTier::parse(value);
        }
    }
    None
}

pub(super) fn resolve_project_context(cwd: &str) -> Result<Option<ProjectContext>, EngineError> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| EngineError::Session(format!("tokio runtime: {e}")))?;
    rt.block_on(async {
        let db = kavach_surreal::open_default()
            .await
            .map_err(|e| EngineError::Session(format!("open db: {e}")))?;
        let Some(project) = kavach_surreal::projects::find_by_path(&db, cwd)
            .await
            .map_err(|e| EngineError::Session(format!("find project: {e}")))?
        else {
            return Ok(None);
        };
        let Some(project_id) = project.id.as_ref() else {
            return Ok(None);
        };
        let mut rows = Vec::with_capacity(64);
        for table in ["app_spec", "decision", "roadmap"] {
            let table_rows = kavach_surreal::read::list_by_project(&db, table, project_id)
                .await
                .map_err(|e| EngineError::Session(format!("query {table}: {e}")))?;
            for r in table_rows {
                rows.push((r.entry_key, r.content));
            }
        }
        let tier = rows
            .iter()
            .find(|(k, _)| k == "workflow.tier.current")
            .and_then(|(_, c)| parse_tier_from_content(c))
            .unwrap_or(kavach_types::ProjectTier::Refactor);
        Ok(Some(ProjectContext {
            slug: project.slug,
            tier,
            rows,
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::parse_tier_from_content;

    #[test]
    fn test_parse_tier() {
        assert_eq!(
            parse_tier_from_content("tier=feature; reason=x"),
            Some(kavach_types::ProjectTier::Feature)
        );
        assert_eq!(parse_tier_from_content("nothing here"), None);
    }
}
