//! Reversible CCR (Compress-Cache-Retrieve) File Store.
use std::fs;
use std::io;
use std::path::PathBuf;

#[cfg(test)]
#[path = "ccr_test.rs"]
mod tests;

/// Threshold in bytes before triggering CCR caching (~1,500 tokens).
pub const CCR_THRESHOLD_BYTES: usize = 6_000;

/// Return directory for storing CCR payload blobs.
#[must_use]
pub fn ccr_cache_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".kavach")
        .join("cache")
        .join("ccr")
}

/// Store original content in CCR cache file keyed by its hash.
///
/// # Errors
/// Returns `io::Error` if directory creation or file writing fails.
pub fn store_blob(hash: &str, content: &[u8]) -> io::Result<PathBuf> {
    let dir = ccr_cache_dir();
    fs::create_dir_all(&dir)?;
    let blob_path = dir.join(format!("{hash}.blob"));
    fs::write(&blob_path, content)?;
    Ok(blob_path)
}

/// Retrieve raw original content from CCR cache by full or prefix hash.
///
/// # Errors
/// Returns `io::Error` if file is not found or cannot be read.
pub fn load_blob(hash_query: &str) -> io::Result<String> {
    let dir = ccr_cache_dir();
    let exact_path = dir.join(format!("{hash_query}.blob"));
    if exact_path.exists() {
        return fs::read_to_string(&exact_path);
    }

    // Support prefix matching if query is a prefix of at least 8 chars.
    if hash_query.len() >= 8 && dir.exists() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(hash_query) && name_str.ends_with(".blob") {
                return fs::read_to_string(entry.path());
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("CCR blob not found for query: {hash_query}"),
    ))
}

/// Truncate long content keeping header, first lines, anomalies, and tail.
#[must_use]
pub fn truncate_lines(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 60 {
        return text.to_owned();
    }

    let head_count = 25.min(lines.len());
    let head = lines.get(..head_count).unwrap_or_default();

    let tail_count = 25.min(lines.len().saturating_sub(head_count));
    let tail_start = lines.len().saturating_sub(tail_count);
    let tail = lines.get(tail_start..).unwrap_or_default();

    // Scan middle slice for anomaly/error lines.
    let middle = lines.get(head_count..tail_start).unwrap_or_default();
    let mut anomalies = Vec::new();
    for line in middle {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error")
            || lower.contains("fail")
            || lower.contains("panic")
            || lower.contains("warning")
            || lower.contains("[rca]")
        {
            anomalies.push(*line);
            if anomalies.len() >= 20 {
                break;
            }
        }
    }

    let omitted = middle.len().saturating_sub(anomalies.len());
    let mut out = head.join("\n");
    if !anomalies.is_empty() {
        out.push_str("\n... [anomalies detected] ...\n");
        out.push_str(&anomalies.join("\n"));
    }
    if omitted > 0 {
        use std::fmt::Write as _;
        write!(out, "\n... ({omitted} lines omitted) ...\n").ok();
    }
    out.push_str(&tail.join("\n"));
    out
}

/// Process payload: if oversized, cache to disk and return truncated preview with retrieve marker.
#[must_use]
pub fn process_ccr(text: &str) -> (String, Option<String>) {
    if text.len() <= CCR_THRESHOLD_BYTES {
        return (text.to_owned(), None);
    }

    let hash = blake3::hash(text.as_bytes()).to_hex().to_string();
    if store_blob(&hash, text.as_bytes()).is_err() {
        return (text.to_owned(), None);
    }

    let truncated = truncate_lines(text);
    let est_tokens = text.len().checked_div(4).unwrap_or(0);
    let preview = format!(
        "{truncated}\n\n[KAVACH CCR: Original (~{est_tokens} tokens) cached. Retrieve if needed: <<kavach_ccr:{hash}>>. Run: kavach retrieve {hash}]\n"
    );

    (preview, Some(hash))
}
