use super::{crates_index_url, parse_versions, prompt_mentions};

const SAMPLE: &str = "\
[[package]]
name = \"surrealdb\"
version = \"3.1.4\"

[[package]]
name = \"tokio\"
version = \"1.47.1\"
";

#[test]
fn parses_name_version_pairs() {
    let v = parse_versions(SAMPLE);
    assert!(v.contains(&("surrealdb".to_owned(), "3.1.4".to_owned())));
    assert!(v.contains(&("tokio".to_owned(), "1.47.1".to_owned())));
}

#[test]
fn whole_token_match_only() {
    assert!(prompt_mentions("upgrade surrealdb please", "surrealdb"));
    assert!(prompt_mentions("using surrealdb 3.1.4 not 2.x", "surrealdb"));
    // substring must NOT match
    assert!(!prompt_mentions("surrealdberg is not the crate", "surrealdb"));
}

#[test]
fn hyphen_underscore_insensitive() {
    assert!(prompt_mentions("the tokio_util helper", "tokio-util"));
    assert!(prompt_mentions("the tokio-util helper", "tokio_util"));
}

#[test]
fn no_mention_no_pin() {
    assert!(!prompt_mentions("just refactor the parser", "surrealdb"));
}

#[test]
fn sparse_index_url_buckets_by_name_length() {
    // 1/2/3-char buckets, then the two-pair bucket. SOURCE: crates.io index protocol.
    assert_eq!(crates_index_url("a"), "https://index.crates.io/1/a");
    assert_eq!(crates_index_url("up"), "https://index.crates.io/2/up");
    assert_eq!(crates_index_url("sd"), "https://index.crates.io/2/sd");
    assert_eq!(crates_index_url("rga"), "https://index.crates.io/3/r/rga");
    assert_eq!(crates_index_url("surrealdb"), "https://index.crates.io/su/rr/surrealdb");
    assert_eq!(crates_index_url("tokio"), "https://index.crates.io/to/ki/tokio");
}
