// split: intentional - granular delete operations (single key or category-wide)
// Preferred over wipe_project for surgical record removal.
use crate::error::{Error, Result};
use crate::db::DbStore;
use crate::db::entries::EntryStore;

#[derive(Debug)]
#[non_exhaustive]
pub struct DeleteReport {
    pub project_slug: String,
    pub category: String,
    pub key: Option<String>,
    pub count: usize,
}

const VALID_CATEGORIES: &[&str] = &["decision", "research", "pattern", "roadmap", "app_spec"];

fn validate_category(category: &str) -> Result<()> {
    if VALID_CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err(Error::RecordNotFound(format!(
            "invalid category '{category}' — must be one of: {}",
            VALID_CATEGORIES.join(", ")
        )))
    }
}

async fn get_project(db: &DbStore, project_slug: &str) -> Result<String> {
    let q = "SELECT slug FROM projects WHERE slug = ? LIMIT 1";
    let mut rows = db
        .exec_params(q, [project_slug.to_owned()])
        .await
        .map_err(Error::Db)?;
    let Some(row) = rows.next().await.map_err(|e| Error::Db(e.into()))? else {
        return Err(Error::RecordNotFound(format!("project '{project_slug}'")));
    };
    Ok(row.get::<String>(0).unwrap_or_default())
}

/// Delete a single record by key — atomically removes the document row,
/// the graph entity (`entity_type`='memory'), and connected edges in a
/// single transaction.
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn delete_by_key(
    db: &DbStore,
    project_slug: &str,
    category: &str,
    key: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let _pid = get_project(db, project_slug).await?;
    let count = db
        .delete_by_key(project_slug, category, key)
        .await
        .map_err(Error::Db)?;
    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(key.to_owned()),
        count: usize::try_from(count).map_or(usize::MAX, |c| c),
    })
}

/// Preview delete of a single record (dry-run).
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn preview_delete_by_key(
    db: &DbStore,
    project_slug: &str,
    category: &str,
    key: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let _pid = get_project(db, project_slug).await?;
    let entries = db
        .list_by_category(project_slug, category)
        .await
        .map_err(Error::Db)?;
    let count = entries.iter().filter(|e| e.entry_key == key).count();
    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(key.to_owned()),
        count,
    })
}

/// Delete all records in a category for a project.
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn delete_category(
    db: &DbStore,
    project_slug: &str,
    category: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let _pid = get_project(db, project_slug).await?;
    let entries = db
        .list_by_category(project_slug, category)
        .await
        .map_err(Error::Db)?;
    let count = entries.len();
    for entry in entries {
        db.delete_by_key(project_slug, category, &entry.entry_key)
            .await
            .map_err(Error::Db)?;
    }
    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: None,
        count,
    })
}

/// Count records in `category` whose `entry_key` starts with `key_prefix`, for the
/// shared count step of both the preview and the apply path.
async fn count_by_key_prefix(
    db: &DbStore,
    project_slug: &str,
    category: &str,
    key_prefix: &str,
) -> Result<usize> {
    let entries = db
        .list_by_category(project_slug, category)
        .await
        .map_err(Error::Db)?;
    Ok(entries
        .iter()
        .filter(|e| e.entry_key.starts_with(key_prefix))
        .count())
}

/// Preview a prefix delete (dry-run): report how many rows in `category` have an
/// `entry_key` starting with `key_prefix`, without deleting anything.
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn preview_delete_by_key_prefix(
    db: &DbStore,
    project_slug: &str,
    category: &str,
    key_prefix: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let _pid = get_project(db, project_slug).await?;
    let count = count_by_key_prefix(db, project_slug, category, key_prefix).await?;
    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(format!("{key_prefix}*")),
        count,
    })
}

/// Delete every record in a category whose `entry_key` starts with `key_prefix`.
///
/// E.g. purge all `heal.incident.loophole-*` cards in one transaction. Atomic:
/// document rows + matching graph entities in a single BEGIN/COMMIT.
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn delete_by_key_prefix(
    db: &DbStore,
    project_slug: &str,
    category: &str,
    key_prefix: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let _pid = get_project(db, project_slug).await?;
    let count = count_by_key_prefix(db, project_slug, category, key_prefix).await?;
    let entries = db
        .list_by_category(project_slug, category)
        .await
        .map_err(Error::Db)?;
    for entry in entries {
        if entry.entry_key.starts_with(key_prefix) {
            db.delete_by_key(project_slug, category, &entry.entry_key)
                .await
                .map_err(Error::Db)?;
        }
    }
    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(format!("{key_prefix}*")),
        count,
    })
}

/// Preview delete of all records in a category (dry-run).
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn preview_delete_category(
    db: &DbStore,
    project_slug: &str,
    category: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let _pid = get_project(db, project_slug).await?;
    let entries = db
        .list_by_category(project_slug, category)
        .await
        .map_err(Error::Db)?;
    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: None,
        count: entries.len(),
    })
}
