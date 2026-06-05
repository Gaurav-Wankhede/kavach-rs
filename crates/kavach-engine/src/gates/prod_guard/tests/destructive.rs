//! `check_prod_destructive` HARD-BLOCK tier tests.
use crate::gates::prod_guard::destructive::check_prod_destructive;

#[test]
fn test_drop_database_blocked() {
    assert!(check_prod_destructive("psql -c drop database production").is_some());
    assert!(check_prod_destructive("DROP DATABASE myapp").is_some());
    // Staging/UAT/dev are also prod-class — must block without manual confirm
    assert!(check_prod_destructive("DROP DATABASE myapp_staging").is_some());
    assert!(check_prod_destructive("DROP DATABASE myapp_uat").is_some());
    assert!(check_prod_destructive("DROP DATABASE myapp_dev").is_some());
    // Only loopback or explicit *_test suffixes pass
    assert!(check_prod_destructive("psql localhost -c drop database test_db").is_none());
    assert!(check_prod_destructive("drop database myapp_test").is_none());
}

#[test]
fn test_volume_delete_blocked() {
    assert!(check_prod_destructive("railway volume delete vol_123").is_some());
    assert!(check_prod_destructive("fly volumes destroy").is_some());
    assert!(check_prod_destructive("aws ebs volume delete").is_some());
}

#[test]
fn test_cloud_db_delete_blocked() {
    assert!(check_prod_destructive("aws rds delete-db-instance --db-id prod").is_some());
    assert!(check_prod_destructive("gcloud sql instances delete prod-db").is_some());
    assert!(check_prod_destructive("heroku pg:reset DATABASE_URL").is_some());
    assert!(check_prod_destructive("doctl databases delete db-123").is_some());
    // Dry-run allowed
    assert!(check_prod_destructive("aws rds delete-db-instance --dry-run").is_none());
}

#[test]
fn test_iac_destroy_blocked() {
    assert!(check_prod_destructive("terraform destroy -auto-approve").is_some());
    assert!(check_prod_destructive("pulumi destroy --yes").is_some());
    assert!(check_prod_destructive("cdk destroy --all").is_some());
    // Targeted destroy allowed
    assert!(check_prod_destructive("terraform destroy -target module.test").is_none());
}

#[test]
fn test_safe_commands_not_blocked() {
    assert!(check_prod_destructive("cargo test").is_none());
    assert!(check_prod_destructive("git push origin main").is_none());
    assert!(check_prod_destructive("aws s3 ls").is_none());
    assert!(check_prod_destructive("terraform plan").is_none());
}
