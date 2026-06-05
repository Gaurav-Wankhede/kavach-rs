//! Pattern tables for `irreversible_guard`.
use regex::Regex;
use std::sync::OnceLock;

pub(super) struct Rule {
    pub re: &'static Regex,
    pub pattern: &'static str,
    pub fix: &'static str,
}

macro_rules! static_regex {
    ($fn_name:ident, $pattern:expr) => {
        fn $fn_name() -> &'static Regex {
            static CELL: OnceLock<Regex> = OnceLock::new();
            CELL.get_or_init(|| Regex::new($pattern).unwrap_or_else(|_| Regex::new(".+").unwrap()))
        }
    };
}

static_regex!(re_drop, r"(?i)\bDROP\s+(?:TABLE|DATABASE|SCHEMA|INDEX)\b");
static_regex!(re_trunc, r"(?i)\bTRUNCATE\s+TABLE\b");
static_regex!(re_del, r"(?i)\bDELETE\s+FROM\s+\w+\s*(?:;|$)");
static_regex!(re_alter, r"(?i)\bALTER\s+TABLE\s+\w+\s+DROP\s+COLUMN\b");
static_regex!(re_etc, r"^/etc/.*");
static_regex!(re_usr, r"^/usr/.*");
static_regex!(re_sys, r"^/System/.*");
static_regex!(re_ssh, r".*\.ssh/.*");
static_regex!(re_aws, r".*\.aws/credentials.*");
static_regex!(re_mig, r"/migrations?/down/|_down\.sql$");

static SQL_TBL: OnceLock<Vec<Rule>> = OnceLock::new();
static PATH_TBL: OnceLock<Vec<(&'static Regex, &'static str, &'static str)>> = OnceLock::new();

pub(super) fn sql_rules() -> &'static [Rule] {
    SQL_TBL.get_or_init(|| vec![
        Rule { re: re_drop(), pattern: "SQL DROP — irreversible schema deletion",
            fix: "Wrap in transaction with backup step. Add IF EXISTS guard. Confirm production target." },
        Rule { re: re_trunc(), pattern: "SQL TRUNCATE — irreversible row purge",
            fix: "Use DELETE FROM with WHERE to allow point-in-time recovery, OR backup first." },
        Rule { re: re_del(), pattern: "SQL DELETE without WHERE — purges entire table",
            fix: "Add WHERE clause OR explicitly TRUNCATE if intentional." },
        Rule { re: re_alter(), pattern: "ALTER TABLE DROP COLUMN — data loss",
            fix: "Rename + soft-delete: ALTER … RENAME TO deprecated_<col>; drop in later migration." },
    ])
}

pub(super) fn path_rules() -> &'static [(&'static Regex, &'static str, &'static str)] {
    PATH_TBL.get_or_init(|| {
        vec![
            (
                re_etc(),
                "Write to /etc — system config",
                "Use a per-user override path or a packaged config dir.",
            ),
            (
                re_usr(),
                "Write to /usr — system binaries/libs",
                "Install into ~/.local/ or a versioned prefix.",
            ),
            (
                re_sys(),
                "Write to /System — macOS system tree",
                "Forbidden under SIP. Use ~/Library/Application Support.",
            ),
            (
                re_ssh(),
                "Write inside ~/.ssh — auth keys",
                "Confirm key rotation; never overwrite existing private keys without backup.",
            ),
            (
                re_aws(),
                "Write to AWS credentials",
                "Use AWS_PROFILE env or aws-vault; never write raw creds to disk.",
            ),
            (
                re_mig(),
                "Migration down file — destructive by design",
                "Verify reversal path is tested in staging; require explicit operator ack.",
            ),
        ]
    })
}
