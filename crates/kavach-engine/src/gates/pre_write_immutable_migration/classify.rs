//! Migration-file path classifier (no regex dep, O(L) single walk).
//!
//! Matches `.../migrations/<digits>_<snake_lower>.sql` and the
//! `.../migrations_<suffix>/...` variant. Cross-platform: both POSIX `/` and
//! Windows `\` separators are accepted (P1: gate silent-passed on Windows).
//! SOURCE: `decision:rca.windows_path_separator_p1`;
//! <https://doc.rust-lang.org/std/path/fn.is_separator.html>.
use std::path::Path;

/// True iff the path tail has the canonical sqlx migration shape.
pub(super) fn is_migration_file(target_path: &str) -> bool {
    if !Path::new(target_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("sql"))
    {
        return false;
    }
    let segments: Vec<&str> = target_path.split(['/', '\\']).collect();
    if segments.len() < 2 {
        return false;
    }
    let Some(file) = segments.last() else {
        return false;
    };
    let Some(parent) = segments.get(segments.len().saturating_sub(2)) else {
        return false;
    };
    if !parent_is_migrations(parent) {
        return false;
    }
    let Some(stem) = file.strip_suffix(".sql") else {
        return false;
    };
    stem_is_versioned(stem)
}

/// `migrations` or `migrations_<ascii-lower-suffix>`.
fn parent_is_migrations(parent: &str) -> bool {
    parent == "migrations"
        || (parent.starts_with("migrations_")
            && parent
                .trim_start_matches("migrations_")
                .chars()
                .all(|c| c.is_ascii_lowercase()))
}

/// `<digits>_<snake_lower-with-digits>` stem shape.
fn stem_is_versioned(stem: &str) -> bool {
    let Some(underscore) = stem.find('_') else {
        return false;
    };
    let (num, rest) = stem.split_at(underscore);
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let Some(body) = rest.get(1..) else {
        return false;
    };
    !body.is_empty()
        && body
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
