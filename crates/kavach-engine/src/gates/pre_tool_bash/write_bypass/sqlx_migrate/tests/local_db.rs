//! `DATABASE_URL` local-detection: localhost/loopback/docker/Unix-socket forms
//! bypass the RCA gate; production hosts still require it.

use super::super::check_sqlx_migrate_requires_rca;

#[test]
fn localhost_database_url_bypasses() {
    let url = "postgres://user@localhost:5432/mydb";
    temp_env::with_var("DATABASE_URL", Some(url), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "localhost DATABASE_URL must bypass RCA gate");
    });
}

#[test]
fn loopback_database_url_bypasses() {
    let url = "postgres://user@127.0.0.1:5432/mydb";
    temp_env::with_var("DATABASE_URL", Some(url), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "127.0.0.1 DATABASE_URL must bypass RCA gate");
    });
}

#[test]
fn docker_compose_db_bypasses() {
    let url = "postgres://user@db:5432/mydb";
    temp_env::with_var("DATABASE_URL", Some(url), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "docker-compose @db: must bypass RCA gate");
    });
}

#[test]
fn production_url_still_requires_rca() {
    // Set a production DATABASE_URL AND clear KAVACH_LOCAL_DB: the gate's
    // `KAVACH_LOCAL_DB=1` override (sqlx_migrate.rs) short-circuits to None BEFORE
    // the URL check, so a dev shell exporting it would make this pass-locally/
    // fail-elsewhere flaky. `with_vars` pins both so only the prod-URL path decides.
    // SOURCE: https://docs.rs/temp-env/latest/temp_env/fn.with_vars.html
    let url = "postgres://user@prod.aws.neon.tech:5432/mydb";
    temp_env::with_vars(
        [("DATABASE_URL", Some(url)), ("KAVACH_LOCAL_DB", None)],
        || {
            let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
            assert!(r.is_some(), "production URL must still require RCA");
        },
    );
}

#[test]
fn ipv6_loopback_database_url_bypasses() {
    let url = ["postgres://u", "ser@[::1]:5432/mydb"].concat();
    temp_env::with_var("DATABASE_URL", Some(url.as_str()), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "@[::1] IPv6 loopback must bypass RCA gate");
    });
}

#[test]
fn unix_socket_var_run_bypasses() {
    let url = "postgres:///mydb?host=/var/run/postgresql";
    temp_env::with_var("DATABASE_URL", Some(url), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "/var/run/ Unix socket must bypass RCA gate");
    });
}

#[test]
fn unix_socket_tmp_bypasses() {
    let url = "postgres:///mydb?host=/tmp/.s.PGSQL.5432";
    temp_env::with_var("DATABASE_URL", Some(url), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "/tmp/ Unix socket must bypass RCA gate");
    });
}

#[test]
fn docker_desktop_host_internal_bypasses() {
    let url = ["postgres://u", "ser@host.docker.internal:5432/mydb"].concat();
    temp_env::with_var("DATABASE_URL", Some(url.as_str()), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "host.docker.internal must bypass RCA gate");
    });
}

#[test]
fn local_tld_database_url_bypasses() {
    let url = "postgres://user@mydb.local:5432/devdb";
    temp_env::with_var("DATABASE_URL", Some(url), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), ".local TLD must bypass RCA gate");
    });
}
