// ARCH: DotenvFilenameAndDownstream
// PROBLEM_CLASS: filename_pattern_detection + binary_allowlist
// REJECTED: [{"name":"glob-match","reason":"adds dep, slower than substring scan"},{"name":"regex","reason":"compile cost per call"},{"name":"hardcoded .env only","reason":"misses .env.local, .envrc, .env.production etc."}]
// TIME: O(n*v) where n = command length, v = ~7 variants | SPACE: O(1) const slice
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: Substring scan is approximate but covers Astro/Next/Vite/Nuxt
//           conventions. False negative on exotic filenames falls back to ".env".
// BENCHMARK: existing env_guard tests preserved across split.
// PATTERN: filename_pattern + binary_allowlist | SCOPE: pre_tool_bash | CAP: AP
// FAILURE_MODE: detect_env_filename returns ".env" fallback if nothing matches —
//               error message stays generic but blocking still works.
//               is_safe_downstream is fail-OPEN for genuine runners: an unknown
//               task-runner that consumes env silently (just/make/sqlx/unknown CLI)
//               is ALLOWED; only an EXPLICIT env-value print (echo/printf/printenv/
//               env/set/export, a raw `bash -c`, or python) or a destructive psql
//               is rejected. A hygiene heuristic must never hard-block a real
//               migration command — that strangles the autonomous loop.
//
// Extracted from env_guard.rs (split-env-guard-microservices roadmap, May 2026).
//
// `downstream` holds the post-source binary allowlist; `filename` detects the
// `.env*` filename for error messages.
mod downstream;
mod filename;
mod target;

#[cfg(test)]
mod tests;

pub(crate) use downstream::is_safe_downstream;
pub(crate) use filename::detect_env_filename;
pub(crate) use target::targets_dotenv_file;
