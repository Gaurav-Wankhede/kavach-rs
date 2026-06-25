use std::path::Path;

/// Sanitize a file path and verify it falls within allowed base directories.
///
/// # Errors
///
/// Returns an error if the path is empty, contains path traversal sequences (".."),
/// cannot be canonicalized (ENOENT, EACCES, ELOOP), or falls outside allowed directories.
pub fn sanitize_path(path: &str, bases: &[&str]) -> Result<String, String> {
	if path.is_empty() {
		return Err("empty path".into());
	}
	let cleaned = Path::new(path)
		.components()
		.collect::<std::path::PathBuf>()
		.to_string_lossy()
		.to_string();
	if cleaned.contains("..") {
		return Err(format!("path traversal detected: {path}"));
	}
	if bases.is_empty() {
		return Ok(cleaned);
	}
	let abs =
		std::fs::canonicalize(&cleaned).map_err(|e| format!("failed to resolve {path}: {e}"))?;
	let a = abs.to_string_lossy();
	for b in bases {
		if let Ok(ab) = std::fs::canonicalize(b)
			&& a.starts_with(&*ab.to_string_lossy())
		{
			return Ok(cleaned);
		}
	}
	Err(format!("path outside allowed dirs: {path}"))
}

/// Validate that an identifier contains only ASCII alphanumerics, underscores, and hyphens.
///
/// # Errors
///
/// Returns an error if the identifier is empty or contains invalid characters.
pub fn validate_identifier(name: &str) -> Result<(), String> {
	if name.is_empty() {
		return Err("empty identifier".into());
	}
	for ch in name.chars() {
		if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' {
			return Err(format!("invalid char: {ch}"));
		}
	}
	Ok(())
}
