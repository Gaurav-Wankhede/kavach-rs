//! `check_prod_ops` soft-warning tests.
use crate::gates::prod_guard::ops::check_prod_ops;

#[test]
fn test_psql_prod_migration_warned() {
    assert!(check_prod_ops("psql \"$DATABASE_URL\" -f migrations/001.sql").is_some());
    assert!(check_prod_ops("source .env && psql $DB -f migrate.sql").is_some());
}

#[test]
fn test_psql_local_ok() {
    assert!(check_prod_ops("psql localhost -f test.sql").is_none());
    assert!(check_prod_ops("psql 127.0.0.1:5432 -f test.sql").is_none());
}

#[test]
fn test_psql_read_ok() {
    assert!(check_prod_ops("psql \"$DATABASE_URL\" -c \"SELECT 1\"").is_none());
}

#[test]
fn test_doctl_create_warned() {
    assert!(check_prod_ops("doctl apps create --spec app.yaml").is_some());
    assert!(check_prod_ops("doctl apps update $ID --spec app.yaml").is_some());
}

#[test]
fn test_git_push_main_warned() {
    assert!(check_prod_ops("git push origin main").is_some());
    assert!(check_prod_ops("git push origin master").is_some());
}

#[test]
fn test_git_push_branch_ok() {
    assert!(check_prod_ops("git push origin feature/foo").is_none());
}

#[test]
fn test_terraform_warned() {
    assert!(check_prod_ops("terraform apply -auto-approve").is_some());
}

#[test]
fn test_normal_commands_ok() {
    assert!(check_prod_ops("cargo test").is_none());
    assert!(check_prod_ops("git status").is_none());
    assert!(check_prod_ops("ls -la").is_none());
}
