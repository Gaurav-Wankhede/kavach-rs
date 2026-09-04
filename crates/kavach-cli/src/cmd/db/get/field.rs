//! Field extractor for `kavach db get --field <name>`.

#[cfg(test)]
#[path = "field_test.rs"]
mod tests;

pub(super) struct FieldEntryView<'a> {
    pub key: &'a str,
    pub category: &'a str,
    pub title: &'a str,
    pub status: &'a str,
    pub content: Option<&'a str>,
    pub access_count: i64,
    pub exec_prompt: Option<&'a str>,
    pub priority: Option<i64>,
    pub lane: Option<&'a str>,
    pub tags: Option<&'a [String]>,
}

pub(super) fn extract_field(
    view: &FieldEntryView<'_>,
    field: &str,
    compact: bool,
) -> Result<String, String> {
    match field.trim().to_ascii_lowercase().as_str() {
        "content" => {
            let raw = view.content.unwrap_or_default();
            if compact {
                Ok(kavach_toon::compact::compress(raw, kavach_toon::compact::Level::Lite))
            } else {
                Ok(raw.to_owned())
            }
        }
        "title" => Ok(view.title.to_owned()),
        "status" | "entry_status" => Ok(view.status.to_owned()),
        "key" | "entry_key" => Ok(view.key.to_owned()),
        "category" => Ok(view.category.to_owned()),
        "access" | "access_count" => Ok(view.access_count.to_string()),
        "exec_prompt" => {
            let prompt = view.exec_prompt.unwrap_or_default();
            if compact {
                Ok(kavach_toon::compact::compress(prompt, kavach_toon::compact::Level::Lite))
            } else {
                Ok(prompt.to_owned())
            }
        }
        "priority" => Ok(view.priority.map_or_else(String::new, |p| p.to_string())),
        "lane" => Ok(view.lane.unwrap_or_default().to_owned()),
        "tags" => Ok(view.tags.map_or_else(String::new, |t| t.join(", "))),
        unknown => Err(format!(
            "unknown field '{unknown}'. Supported: content, title, status, key, category, access, exec_prompt, priority, lane, tags"
        )),
    }
}
