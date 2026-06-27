use super::RoleQuery;

#[test]
fn parse_full_query() {
    let json = r#"{
        "role": "db_user",
        "value_regex": "^postgres://",
        "consumed_by": ["app1", "app2"],
        "env_key_hints": ["DB_URL", "DATABASE_URI"],
        "name_aliases": ["postgres_creds", "pg_auth"]
    }"#;
    let result = RoleQuery::parse(json).expect("parse failed");
    assert_eq!(result.role, "db_user");
    assert_eq!(result.value_regex, Some("^postgres://".to_owned()));
    assert_eq!(result.consumed_by, vec!["app1", "app2"]);
    assert_eq!(result.env_key_hints, vec!["DB_URL", "DATABASE_URI"]);
    assert_eq!(result.name_aliases, vec!["postgres_creds", "pg_auth"]);
}

#[test]
fn parse_empty_object() {
    let json = "{}";
    let result = RoleQuery::parse(json).expect("parse failed");
    assert_eq!(result.role, "");
    assert_eq!(result.value_regex, None);
    assert!(result.consumed_by.is_empty());
    assert!(result.env_key_hints.is_empty());
    assert!(result.name_aliases.is_empty());
}

#[test]
fn parse_partial_only_value_regex() {
    let json = r#"{"value_regex":"^postgres://"}"#;
    let result = RoleQuery::parse(json).expect("parse failed");
    assert_eq!(result.role, "");
    assert_eq!(result.value_regex, Some("^postgres://".to_owned()));
    assert!(result.consumed_by.is_empty());
    assert!(result.env_key_hints.is_empty());
    assert!(result.name_aliases.is_empty());
}

#[test]
fn parse_invalid_json() {
    let json = "{not json";
    let result = RoleQuery::parse(json);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.starts_with("invalid role-query JSON:"));
}

#[test]
fn parse_arbitrary_name_alias() {
    let json = r#"{"name_aliases": ["ABC", "DEF", "GHI"]}"#;
    let result = RoleQuery::parse(json).expect("parse failed");
    assert!(result.name_aliases.contains(&"ABC".to_owned()));
    assert_eq!(result.name_aliases, vec!["ABC", "DEF", "GHI"]);
}
