//! SOFT-WARNING tier: high-risk but not immediately destructive prod ops.
use super::detect::detection_view;

/// SOFT WARNING: Returns Some(warning) for high-risk but not immediately
/// destructive ops — psql on prod, doctl/aws/gcloud create, git push to
/// main/master, DB migrations on non-local DBs. Uses the same DB-client-aware
/// `detection_view` as `check_prod_destructive`.
pub(crate) fn check_prod_ops(command: &str) -> Option<String> {
    let lower = detection_view(command);

    // Production database operations
    if lower.contains("psql")
        && !lower.contains("localhost")
        && !lower.contains("127.0.0.1")
        && (lower.contains("-f ")
            || lower.contains("migrate")
            || lower.contains("alter ")
            || lower.contains("drop ")
            || lower.contains("create table"))
    {
        return Some(
            "[PROD_OPS_WARNING] Production database modification detected. \
             Confirm with user before applying migrations or schema changes \
             to non-local databases."
                .to_owned(),
        );
    }

    // Cloud infrastructure creation
    if lower.contains("doctl apps create")
        || lower.contains("doctl apps update")
        || lower.contains("aws ecs create")
        || lower.contains("gcloud run deploy")
        || lower.contains("terraform apply")
        || lower.contains("pulumi up")
    {
        return Some(
            "[PROD_OPS_WARNING] Cloud infrastructure creation/modification detected. \
             Confirm with user before creating or modifying cloud resources."
                .to_owned(),
        );
    }

    // Git push to main/master
    if lower.contains("git push") && (lower.contains(" main") || lower.contains(" master")) {
        return Some(
            "[PROD_OPS_WARNING] Pushing to main/master branch detected. \
             Confirm with user before pushing to production branch."
                .to_owned(),
        );
    }

    None
}
