use super::{parse_pyproject_deps, parse_requirements_txt};

#[test]
fn parses_requirements_txt_with_pin_operator() {
    let sample = "requests==2.31.0\nflask>=2.0.0\n";
    let deps = parse_requirements_txt(sample);
    assert!(deps.contains(&("requests".to_owned(), "2.31.0".to_owned())));
    assert!(deps.contains(&("flask".to_owned(), ">=2.0.0".to_owned())));
}

#[test]
fn skips_comment_lines_in_requirements() {
    let sample = "# This is a comment\nrequests==2.31.0\n";
    let deps = parse_requirements_txt(sample);
    assert_eq!(deps.len(), 1);
    assert!(deps.contains(&("requests".to_owned(), "2.31.0".to_owned())));
}

#[test]
fn skips_flags_in_requirements() {
    let sample = "-e git+https://github.com/user/repo.git\nrequests==2.31.0\n-r other.txt\n";
    let deps = parse_requirements_txt(sample);
    assert_eq!(deps.len(), 1);
    assert!(deps.contains(&("requests".to_owned(), "2.31.0".to_owned())));
}

#[test]
fn parses_pyproject_deps() {
    let sample = r#"[project]
name = "myapp"
dependencies = ["httpx>=0.27.0", "pydantic==2.0.0"]
"#;
    let deps = parse_pyproject_deps(sample);
    assert!(deps.contains(&("httpx".to_owned(), ">=0.27.0".to_owned())));
    assert!(deps.contains(&("pydantic".to_owned(), "2.0.0".to_owned())));
}

#[test]
fn returns_empty_on_no_deps() {
    assert_eq!(parse_requirements_txt(""), Vec::<(String, String)>::new());
    assert_eq!(
        parse_requirements_txt("# only comments\n"),
        Vec::<(String, String)>::new()
    );
}
