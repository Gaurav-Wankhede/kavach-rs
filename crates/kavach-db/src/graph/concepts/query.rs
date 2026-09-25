// Concept lookups — find-by-name, list, BM25 FTS search.
use crate::error::Result;
use crate::graph::types::Entity;
use crate::query_lex::{Lexer, QueryAst, TokenKind};
use crate::turso::TursoStore;

pub(crate) const SEARCH_LIMIT_MAX: i64 = 200;

fn clamp_limit(limit: usize) -> i64 {
    match i64::try_from(limit) {
        Ok(n) if n > 0 && n <= SEARCH_LIMIT_MAX => n,
        Ok(n) if n > SEARCH_LIMIT_MAX => SEARCH_LIMIT_MAX,
        Ok(_) | Err(_) => SEARCH_LIMIT_MAX,
    }
}

/// Map an `entities` row to a concept `Entity`.
fn entity_from_row(row: &libsql::Row) -> Result<Entity> {
    let props = row
        .get::<Option<String>>(3)?
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| crate::error::Error::Migration(format!("concept props: {e}")))?;
    Ok(Entity {
        id: row.get::<Option<String>>(0)?,
        entity_type: row.get::<String>(1)?,
        name: row.get::<String>(2)?,
        properties: props,
        content_hash: row.get::<Option<String>>(4)?,
        project: row.get::<Option<String>>(5)?,
    })
}

/// Find a concept by canonical name. Returns None if absent.
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn find_concept(db: &TursoStore, name: &str) -> Result<Option<Entity>> {
    let mut resp = db
        .exec_params(
            "SELECT id, entity_type, name, properties, content_hash, project FROM entities \
             WHERE entity_type = 'concept' AND name = ?1 LIMIT 1",
            libsql::params![name.to_owned()],
        )
        .await?;
    resp.next()
        .await?
        .map_or_else(|| Ok(None), |row| entity_from_row(&row).map(Some))
}

/// Lex a concept query to word spans, once per search. Borrowed — no per-word
/// heap. The caller holds `query` alive across row matching.
#[must_use]
pub fn concept_query_words(query: &str) -> Vec<&str> {
    QueryAst::parse(query).words().collect()
}

/// Whole-word concept match with AND semantics (FTS parity).
///
/// Every query word must equal a whole word in `name` or `desc` (ASCII
/// case-insensitive, punctuation-trimmed). Substring matching (`LIKE '%auth%'`
/// hitting "author", or JSON keys like `"desc"` matching the word "desc") is
/// gone; rows are scanned as token spans with zero allocation.
#[must_use]
pub fn concept_text_matches(name: &str, desc: &str, words: &[&str]) -> bool {
    words
        .iter()
        .all(|w| text_has_word(name, w) || text_has_word(desc, w))
}

/// True when `word` (already edge-trimmed by the caller contract below) hits a
/// whole word in `haystack`.
fn text_has_word(haystack: &str, word: &str) -> bool {
    let word = trim_word(word);
    if word.is_empty() {
        return false;
    }
    let mut lexer = Lexer::new(haystack);
    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::EndOfFile {
            return false;
        }
        if trim_word(lexer.text(&token)).eq_ignore_ascii_case(word) {
            return true;
        }
    }
}

/// Strip edge punctuation so `"postgres,"` matches `"postgres"`.
fn trim_word(word: &str) -> &str {
    word.trim_matches(|c: char| !c.is_ascii_alphanumeric())
}

/// Full-text search concepts by name + description via whole-word matching.
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn search_concepts_fts(
    db: &TursoStore,
    query: &str,
    limit: usize,
) -> Result<Vec<Entity>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let words = concept_query_words(trimmed);
    if words.is_empty() {
        return Ok(Vec::new());
    }
    // Fetch a bounded candidate pool (concept population is small); the exact
    // whole-word filter runs in Rust below. `limit == 0` keeps the old
    // clamp-to-200 behavior (uncapped callers).
    let cap = clamp_limit(limit);
    let mut resp = db
        .exec_params(
            "SELECT id, entity_type, name, properties, content_hash, project FROM entities \
             WHERE entity_type = 'concept' ORDER BY name LIMIT ?1",
            libsql::params![cap],
        )
        .await?;
    let mut rows = Vec::new();
    while let Some(row) = resp.next().await? {
        rows.push(entity_from_row(&row)?);
    }
    let want = if limit == 0 {
        rows.len()
    } else {
        limit.min(rows.len())
    };
    let mut out = Vec::new();
    for entity in &rows {
        if out.len() >= want {
            break;
        }
        if concept_text_matches(&entity.name, &concept_search_text(entity), &words) {
            out.push(entity.clone());
        }
    }
    Ok(out)
}

/// Search haystack for one concept: name plus the known text fields. Sources
/// are included (a "github" query should hit a github URL, tokenized); raw
/// JSON keys never are.
fn concept_search_text(entity: &Entity) -> String {
    let mut text = entity.name.clone();
    let mut push = |part: &str| {
        text.push(' ');
        text.push_str(part);
    };
    match entity.properties.as_ref() {
        Some(props) if props.is_object() => {
            if let Some(desc) = props.get("desc").and_then(|d| d.as_str()) {
                push(desc);
            }
            for key in ["tags", "sources"] {
                if let Some(items) = props.get(key).and_then(|v| v.as_array()) {
                    for item in items.iter().filter_map(|v| v.as_str()) {
                        push(item);
                    }
                } else if let Some(single) = props.get(key).and_then(|v| v.as_str()) {
                    push(single);
                }
            }
        }
        Some(props) => push(&props.to_string()),
        None => {}
    }
    text
}

/// List every concept (paginated). Caps at `SEARCH_LIMIT_MAX` per call.
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn list_concepts(db: &TursoStore, limit: usize) -> Result<Vec<Entity>> {
    let limit = clamp_limit(limit);
    let mut resp = db
        .exec_params(
            "SELECT id, entity_type, name, properties, content_hash, project FROM entities \
             WHERE entity_type = 'concept' ORDER BY name LIMIT ?1",
            libsql::params![limit],
        )
        .await?;
    let mut rows = Vec::new();
    while let Some(row) = resp.next().await? {
        rows.push(entity_from_row(&row)?);
    }
    Ok(rows)
}

#[cfg(test)]
#[path = "query_test.rs"]
mod query_test;
