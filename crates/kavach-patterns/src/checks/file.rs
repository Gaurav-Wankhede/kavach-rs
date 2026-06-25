use crate::config::load;
use crate::regex_patterns::fbase;
use std::path::Path;

#[must_use]
pub fn is_sensitive(path: &str) -> bool {
	let g = load();
	let p = path.to_lowercase();
	g.as_ref()
		.is_some_and(|c| c.sensitive.iter().any(|s| p.contains(s)))
}

#[must_use]
pub fn is_code_file(path: &str) -> bool {
	let g = load();
	let p = path.to_lowercase();
	g.as_ref()
		.is_some_and(|c| c.code_exts.iter().any(|e| p.ends_with(e)))
}

#[must_use]
pub fn is_infra_file(path: &str) -> bool {
	let p = path.to_lowercase();
	let b = fbase(path);
	if [
		"dockerfile",
		"makefile",
		"jenkinsfile",
		"caddyfile",
		"nginx.conf",
	]
	.iter()
	.any(|f| b == *f)
		|| b.starts_with("docker-compose")
	{
		return true;
	}
	if [".yml", ".yaml", ".tf", ".tfvars", ".hcl"]
		.iter()
		.any(|e| p.ends_with(e))
	{
		return true;
	}
	if Path::new(p.as_str())
		.extension()
		.is_some_and(|e| e.eq_ignore_ascii_case("toml"))
		&& b != "cargo.toml"
	{
		return true;
	}
	p.contains(".github/") || p.contains(".gitlab-ci")
}

#[must_use]
pub fn is_large_file(path: &str) -> bool {
	let g = load();
	let p = path.to_lowercase();
	g.as_ref()
		.is_some_and(|c| c.large_exts.iter().any(|e| p.ends_with(e)))
}
