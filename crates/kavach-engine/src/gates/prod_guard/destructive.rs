//! HARD-BLOCK tier: irreversible production destruction.
//! SOURCE: Pocket OS incident (April 2026) — AI agent deleted prod DB in 9s
//! <https://blog.railway.com/p/your-ai-wants-to-nuke-your-database>
use super::detect::detection_view;

/// HARD BLOCK: Returns `Some(block_reason)` for destructive production operations.
/// These commands are blocked outright, not just warned.
pub(crate) fn check_prod_destructive(command: &str) -> Option<String> {
    let lower = detection_view(command);

    // Production database DROP/TRUNCATE — only loopback hosts allowed without
    // confirm. Staging/UAT must be treated as prod (code-review feedback).
    let is_explicitly_local = lower.contains("localhost") || lower.contains("127.0.0.1");
    let target_test_db = lower.contains("_test ")
        || lower.contains("_test\"")
        || lower.contains("_test'")
        || lower.ends_with("_test")
        || lower.contains("test_db");
    let allow_destructive = is_explicitly_local || target_test_db;
    if (lower.contains("drop database")
        || lower.contains("drop schema")
        || lower.contains("truncate "))
        && !allow_destructive
    {
        return Some(
            "[DESTRUCTIVE_OP] Production database destruction detected — DROP DATABASE/SCHEMA \
             and TRUNCATE on non-local databases require manual execution with explicit user \
             confirmation -> verify this is NOT a production database, and if intentional, \
             run the command manually in a terminal -> retry."
                .to_owned(),
        );
    }

    // Cloud platform volume/storage deletion (Pocket OS vector)
    if lower.contains("volume delete")
        || lower.contains("volume rm")
        || lower.contains("volumes destroy")
        || lower.contains("volumes delete")
        || lower.contains("storage delete")
        || lower.contains("bucket delete")
    {
        return Some(
            "[DESTRUCTIVE_OP] Cloud storage/volume deletion detected — irreversible, can \
             cause data loss -> verify backups exist elsewhere, confirm this is NOT production \
             storage, and if intentional, run the command manually with explicit confirmation \
             -> retry."
                .to_owned(),
        );
    }

    // Database instance deletion on cloud platforms
    if (lower.contains("rds delete-db")
        || lower.contains("sql instances delete")
        || lower.contains("databases delete")
        || lower.contains("pg:reset"))
        && !lower.contains("--dry-run")
    {
        return Some(
            "BLOCKED: Cloud database deletion detected. \
             This permanently destroys the database and may not be recoverable. \
             FIX: 1) Ensure final snapshots are enabled. \
             2) Verify deletion protection is disabled intentionally. \
             3) If intentional, run the command manually."
                .to_owned(),
        );
    }

    // Terraform/IaC destroy without plan
    if (lower.contains("terraform destroy")
        || lower.contains("pulumi destroy")
        || lower.contains("cdk destroy"))
        && !lower.contains("--dry-run")
        && !lower.contains("-target")
    {
        return Some(
            "BLOCKED: Infrastructure destruction detected. \
             Destroying infrastructure without targeting specific resources is dangerous. \
             FIX: 1) Run with --dry-run first to see what will be destroyed. \
             2) Use -target to destroy specific resources only. \
             3) If intentional, run the command manually."
                .to_owned(),
        );
    }

    None
}
