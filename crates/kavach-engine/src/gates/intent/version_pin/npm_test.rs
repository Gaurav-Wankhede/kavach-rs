use super::parse_package_json;

#[test]
fn parses_dependencies_section() {
    let sample = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.2.0",
    "lodash": "4.17.21"
  }
}"#;
    let deps = parse_package_json(sample);
    assert!(deps.contains(&("react".to_owned(), "^18.2.0".to_owned())));
    assert!(deps.contains(&("lodash".to_owned(), "4.17.21".to_owned())));
}

#[test]
fn parses_dev_dependencies() {
    let sample = r#"{
  "devDependencies": {
    "jest": "^29.0.0",
    "typescript": "5.0.0"
  }
}"#;
    let deps = parse_package_json(sample);
    assert!(deps.contains(&("jest".to_owned(), "^29.0.0".to_owned())));
    assert!(deps.contains(&("typescript".to_owned(), "5.0.0".to_owned())));
}

#[test]
fn returns_empty_on_malformed_json() {
    let bad = "{ invalid json ]";
    assert_eq!(parse_package_json(bad), Vec::<(String, String)>::new());
}

#[test]
fn merges_dependencies_and_dev_dependencies() {
    let sample = r#"{
  "dependencies": {
    "express": "4.18.0"
  },
  "devDependencies": {
    "mocha": "10.0.0"
  }
}"#;
    let deps = parse_package_json(sample);
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&("express".to_owned(), "4.18.0".to_owned())));
    assert!(deps.contains(&("mocha".to_owned(), "10.0.0".to_owned())));
}
