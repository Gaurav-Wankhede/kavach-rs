//! Architecture Research pre-write guard.
//!
//! Blocks writes to Rust files that introduce architectural patterns
//! without prior `/arch` skill invocation this turn, OR auto-injects
//! prior architecture decisions from kavach-db when available.
//!
//! Three outcomes:
//! - `Allow` — no trigger found, or arch skill invoked, or `// ARCH:` comment present
//! - `AutoInject(ctx)` — trigger found, but prior DB decision exists; inject as advisory
//! - `Block(msg)` — trigger found, no prior decision, arch skill not invoked

use kavach_patterns::arch_guard::{ArchGuardOutcome, check as arch_check};

/// Query kavach-rpc daemon for prior architecture decisions for this project.
/// Falls back to None if daemon not running — gate degrades to Block path.
fn load_prior_decision(project_slug: &str, pattern: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({"project": project_slug, "limit": 5});
    let recent: Vec<kavach_surreal::ArchDecision> =
        kavach_rpc::client::call("arch.list_recent", Some(params)).ok()?;
    if recent.is_empty() {
        return None;
    }
    let matched = recent
        .iter()
        .find(|d| d.pattern.contains(pattern) || pattern.contains(&d.pattern));
    let decision = match matched {
        Some(d) => d,
        None => recent.first()?,
    };
    let ctx = format!(
        "[ARCH_AUTO_INJECT]\nstatus: prior_decision_found\n\
         pattern: {}\nscope: {}\ncap: {}\n\
         failure_mode: {}\ntradeoff: {}\n\
         searched: {}-{:02}\nfile: {}\n\
         advisory: Prior decision injected — confirm it still applies or re-run /arch",
        decision.pattern,
        decision.scope,
        decision.cap_choice.as_deref().unwrap_or("N/A"),
        decision.failure_mode,
        decision.tradeoff,
        decision.search_year,
        decision.search_month,
        decision.file_path,
    );
    Some(ctx)
}

/// Outcome of the architecture guard check.
pub(crate) enum ArchPreWriteOutcome {
    /// Write is approved — no action needed.
    Allow,
    /// Write is approved, but inject prior decision as advisory context.
    AutoInject(String),
    /// Write is blocked — invoke /arch skill first.
    Block(String),
}

/// Check whether the write requires prior `/arch` skill invocation.
pub(crate) fn check(
    file_path: &str,
    content: &str,
    arch_satisfied: bool,
    project_slug: &str,
) -> ArchPreWriteOutcome {
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return ArchPreWriteOutcome::Allow;
    }

    let outcome = arch_check(file_path, content, arch_satisfied);

    match outcome {
        ArchGuardOutcome::Allow | ArchGuardOutcome::AllowWithComment => ArchPreWriteOutcome::Allow,
        ArchGuardOutcome::Block(msg) => {
            // Try auto-inject from DB before blocking.
            if let Some(ctx) = load_prior_decision(project_slug, "arch") {
                return ArchPreWriteOutcome::AutoInject(ctx);
            }
            ArchPreWriteOutcome::Block(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_allow(o: &ArchPreWriteOutcome) -> bool {
        matches!(o, ArchPreWriteOutcome::Allow)
    }

    fn is_block(o: &ArchPreWriteOutcome) -> bool {
        matches!(o, ArchPreWriteOutcome::Block(_))
    }

    #[test]
    fn allows_when_satisfied() {
        let code = "let c = distributed_cache::new();";
        assert!(is_allow(&check("src/cache.rs", code, true, "")));
    }

    #[test]
    fn blocks_when_not_satisfied_no_db() {
        let code = "let c = distributed_cache::new();";
        assert!(is_block(&check("src/cache.rs", code, false, "")));
    }

    #[test]
    fn allows_non_rust_file() {
        let code = "const cache = distributed_cache.new();";
        assert!(is_allow(&check("src/cache.ts", code, false, "")));
    }

    #[test]
    fn allows_no_trigger() {
        let code = "fn greet(name: &str) -> String { name.to_string() }";
        assert!(is_allow(&check("src/greet.rs", code, false, "")));
    }
}
