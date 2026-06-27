//! Path classification (POSIX + Windows), non-migration pass-through, and the
//! `.applied` marker heuristic.
use super::applied::is_presumed_applied;
use super::check::check;
use super::classify::is_migration_file;

#[test]
fn test_is_migration_file_matches_canonical_paths() {
    assert!(is_migration_file("migrations/001_init.sql"));
    assert!(is_migration_file("migrations_local/253_bootstrap.sql"));
    assert!(is_migration_file("db/migrations/0042_users.sql"));
    assert!(is_migration_file(
        "/Users/x/project/Backend/migrations_local/250_comms.sql"
    ));
}

#[test]
fn test_is_migration_file_rejects_non_migrations() {
    assert!(!is_migration_file("src/main.rs"));
    assert!(!is_migration_file("migrations/README.md"));
    assert!(!is_migration_file("migrations_local/notes.txt"));
    assert!(!is_migration_file("migrations/init.sql"));
    assert!(!is_migration_file("schema/001_init.sql"));
}

#[test]
fn test_check_passes_non_migration() {
    assert_eq!(check("src/main.rs"), None);
}

#[test]
fn test_is_migration_file_handles_windows_backslash() {
    assert!(is_migration_file(r"C:\project\migrations\001_init.sql"));
    assert!(is_migration_file(
        r"Backend\migrations_local\253_bootstrap.sql"
    ));
    assert!(is_migration_file(r"db/migrations\042_mixed.sql"));
}

#[test]
fn test_is_presumed_applied_via_marker_file() {
    let tmp = std::env::temp_dir().join(format!(
        "kavach_immut_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&tmp).expect("create test dir");
    let migration = tmp.join("001_init.sql");
    std::fs::write(&migration, "-- test").expect("create migration");
    let marker = tmp.join("001_init.sql.applied");
    std::fs::write(&marker, "").expect("create marker");

    assert!(
        is_presumed_applied(&migration),
        ".applied marker must trip the heuristic"
    );

    std::fs::remove_file(&migration).ok();
    std::fs::remove_file(&marker).ok();
    std::fs::remove_dir(&tmp).ok();
}
