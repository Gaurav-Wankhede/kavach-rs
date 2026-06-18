use std::collections::HashMap;
use std::fmt;

use thiserror::Error;

//   {"name":"Result<T,String>","reason":"stringly-typed; callers cannot match variants"},
//   {"name":"Box<dyn std::error::Error>","reason":"erases source type; no #[from]"},
//   {"name":"anyhow::Error","reason":"app-layer ergonomic; lib consumers want typed match"}
// ]
// TIME: O(1) construction | SPACE: O(1) per variant
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: https://oneuptime.com/blog/post/2026-01-25-error-types-thiserror-anyhow-rust/view
#[derive(Debug, Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate error type; callers match variants"
)]
pub enum ToonParseError {
    #[error("empty TOON content")]
    Empty,
}

/// A TOON block with key-value fields and arrays.
#[derive(Debug, Clone, Default)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal construction DTO"
)]
pub struct Block {
    pub name: String,
    pub fields: HashMap<String, String>,
    pub arrays: HashMap<String, Vec<String>>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// A parsed TOON document containing named blocks.
#[derive(Debug, Clone, Default)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal construction DTO"
)]
pub struct Document {
    pub blocks: HashMap<String, Block>,
}

impl Document {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Block> {
        self.blocks.get(name)
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.insert(block.name.clone(), block);
    }
}

/// Parse a TOON string into a Document.
///
/// Recognizes `[BLOCK_NAME]` headers and `key: value` fields.
///
/// # Errors
///
/// Returns [`ToonParseError::Empty`] if content is empty or whitespace-only.
pub fn parse_string(content: &str) -> Result<Document, ToonParseError> {
    if content.trim().is_empty() {
        return Err(ToonParseError::Empty);
    }

    let mut doc = Document::new();
    let mut current_block: Option<Block> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
            if let Some(block) = current_block.take() {
                doc.add_block(block);
            }
            #[expect(
                clippy::string_slice,
                reason = "bounded by len check and bracket guards"
            )]
            let name = trimmed[1..trimmed.len().saturating_sub(1)].to_string();
            current_block = Some(Block {
                name,
                fields: HashMap::new(),
                arrays: HashMap::new(),
            });
        } else if let Some(ref mut block) = current_block
            && let Some((key, value)) = trimmed.split_once(':')
        {
            let key = key.trim().to_owned();
            let value = value.trim().to_owned();
            if !key.is_empty() && !key.starts_with('#') {
                block.fields.insert(key, value);
            }
        }
    }

    if let Some(block) = current_block.take() {
        doc.add_block(block);
    }

    Ok(doc)
}
