use super::{crates_index_url, npm_url, pypi_url};

#[test]
fn npm_url_formats_registry_link() {
    assert_eq!(npm_url("react"), "https://registry.npmjs.org/react");
    assert_eq!(npm_url("lodash"), "https://registry.npmjs.org/lodash");
}

#[test]
fn pypi_url_formats_json_api() {
    assert_eq!(pypi_url("requests"), "https://pypi.org/pypi/requests/json");
    assert_eq!(pypi_url("flask"), "https://pypi.org/pypi/flask/json");
}

#[test]
fn crates_index_url_buckets_by_name_length() {
    assert_eq!(crates_index_url("a"), "https://index.crates.io/1/a");
    assert_eq!(crates_index_url("up"), "https://index.crates.io/2/up");
    assert_eq!(crates_index_url("sd"), "https://index.crates.io/2/sd");
    assert_eq!(crates_index_url("rga"), "https://index.crates.io/3/r/rga");
    assert_eq!(crates_index_url("surrealdb"), "https://index.crates.io/su/rr/surrealdb");
    assert_eq!(crates_index_url("tokio"), "https://index.crates.io/to/ki/tokio");
}
