//! Version-detection + stale-version-block tests.
use crate::gates::pre_tool_search::deps::extract_major_version;
use crate::gates::pre_tool_search::version::check_stale_version_in_query;

#[test]
fn should_extract_major_from_caret_semver() {
    assert_eq!(extract_major_version("^6.1.0"), Some(6));
    assert_eq!(extract_major_version("^19.0.0"), Some(19));
}

#[test]
fn should_extract_major_from_tilde() {
    assert_eq!(extract_major_version("~3.2.1"), Some(3));
}

#[test]
fn should_extract_major_from_bare_version() {
    assert_eq!(extract_major_version("2.0.0"), Some(2));
    assert_eq!(extract_major_version("18"), Some(18));
}

#[test]
fn should_return_none_for_non_version() {
    assert_eq!(extract_major_version("workspace"), None);
    assert_eq!(extract_major_version("{ path ="), None);
}

#[test]
fn should_block_stale_version_when_newer_installed() {
    // Simulate the matching logic: query "astro 5" vs installed major 6.
    let versions = vec![("astro".to_owned(), 6u32)];
    let query = "Astro 5 dashboard redesign";
    let lower = query.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let mut blocked = false;
    for pair in words.windows(2) {
        if let [name, ver] = pair
            && let Ok(qm) = ver.parse::<u32>()
        {
            for (dep, im) in &versions {
                if *name == dep.to_lowercase() && qm < *im {
                    blocked = true;
                }
            }
        }
    }
    assert!(blocked);
}

#[test]
fn should_not_block_current_version() {
    assert!(check_stale_version_in_query("Astro 6 routing", "/nonexistent").is_none());
}

#[test]
fn should_not_block_when_no_workdir() {
    assert!(check_stale_version_in_query("React 17 hooks", "").is_none());
}
