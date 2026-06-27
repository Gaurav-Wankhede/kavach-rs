//! `check` (P0 block) + `format_advisory` (P1) coverage across privilege
//! fields, PII, bool defaults, deny-unknown, and path/test skips.
use super::{check, format_advisory};

const ADMIN_DEFAULT: &str = "#[serde(default)] is_admin: bool,";
const ROLE_DEFAULT: &str = "#[serde(default)] pub role: String,";
const PERM_DEFAULT: &str = "#[serde(default)] pub permission: String,";
const MOD_DEFAULT: &str = "#[serde(default)] is_moderator: bool,";
const OWNER_DEFAULT: &str = "#[serde(default)] is_owner: bool,";
const PAGE_DEFAULT: &str = "#[serde(default)] pub page: u32,";

#[test]
fn should_block_serde_default_on_is_admin() {
    assert!(check("src/handlers/auth.rs", ADMIN_DEFAULT).is_some());
}

#[test]
fn should_block_serde_default_on_role() {
    assert!(check("src/routes/user.rs", ROLE_DEFAULT).is_some());
}

#[test]
fn should_block_serde_default_on_permission() {
    assert!(check("src/api/grant.rs", PERM_DEFAULT).is_some());
}

#[test]
fn should_block_serde_default_on_is_moderator() {
    assert!(check("src/handlers/mod.rs", MOD_DEFAULT).is_some());
}

#[test]
fn should_block_serde_default_on_is_owner() {
    assert!(check("src/api/resource.rs", OWNER_DEFAULT).is_some());
}

#[test]
fn should_allow_when_option_used_instead() {
    assert!(
        check(
            "src/handlers/auth.rs",
            "struct Req { is_admin: Option<bool> }"
        )
        .is_none()
    );
}

#[test]
fn should_allow_serde_default_on_unrelated_field() {
    assert!(check("src/handlers/search.rs", PAGE_DEFAULT).is_none());
}

#[test]
fn should_skip_non_handler_paths() {
    assert!(check("src/models/user.rs", ADMIN_DEFAULT).is_none());
}

#[test]
fn should_skip_test_files() {
    assert!(check("src/handlers/auth.test.rs", ADMIN_DEFAULT).is_none());
}

#[test]
fn should_advise_pii_email_as_plain_string() {
    assert!(format_advisory("src/handlers/profile.rs", "pub email: string,").is_some());
}

#[test]
fn should_advise_pii_ssn_as_plain_string() {
    assert!(format_advisory("src/api/kyc.rs", "pub ssn: string,").is_some());
}

#[test]
fn should_advise_bool_serde_default() {
    assert!(
        format_advisory(
            "src/handlers/settings.rs",
            "#[serde(default)] pub verified: bool,"
        )
        .is_some()
    );
}

#[test]
fn should_advise_missing_deny_unknown_on_auth_struct() {
    assert!(
        format_advisory(
            "src/handlers/auth.rs",
            "struct AuthRequest { username: String }"
        )
        .is_some()
    );
}

#[test]
fn should_not_advise_when_deny_unknown_present() {
    assert!(
        format_advisory(
            "src/handlers/auth.rs",
            "#[serde(deny_unknown_fields)] struct AuthRequest { username: String }"
        )
        .is_none()
    );
}

#[test]
fn should_skip_non_rs_files() {
    assert!(format_advisory("src/auth.ts", "pub email: String").is_none());
}

#[test]
fn should_skip_test_files_in_advisory() {
    assert!(format_advisory("src/handlers/auth.test.rs", "pub email: String").is_none());
}
