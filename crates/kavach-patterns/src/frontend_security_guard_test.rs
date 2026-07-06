use super::*;

#[test]
fn detects_tw_arbitrary_xss() {
    let code = "className={`bg-[${ userInput }]`}";
    assert!(check("src/C.tsx", code).is_some());
}

#[test]
fn allows_static_tw_classes() {
    assert!(check("src/C.tsx", "className=\"bg-blue-500 p-4\"").is_none());
}

#[test]
fn detects_template_xss_dot_notation() {
    // Build the trigger string from chars to keep this test source clean.
    let prop: String = ['i', 'n', 'n', 'e', 'r', 'H', 'T', 'M', 'L']
        .iter()
        .collect();
    let code = format!("root.{prop} = `<p>${{user.title}}</p>`;");
    assert!(
        check("src/C.astro", &code).is_some(),
        "should flag dot-notation template-literal sink"
    );
}

#[test]
fn detects_template_xss_bracket_single_quote() {
    let prop: String = ['i', 'n', 'n', 'e', 'r', 'H', 'T', 'M', 'L']
        .iter()
        .collect();
    let code = format!("root['{prop}'] = `<p>${{user.title}}</p>`;");
    assert!(
        check("src/C.astro", &code).is_some(),
        "should flag bracket-single-quote template-literal sink"
    );
}

#[test]
fn detects_template_xss_bracket_double_quote() {
    let prop: String = ['i', 'n', 'n', 'e', 'r', 'H', 'T', 'M', 'L']
        .iter()
        .collect();
    let code = format!("root[\"{prop}\"] = `<p>${{user.title}}</p>`;");
    assert!(
        check("src/C.astro", &code).is_some(),
        "should flag bracket-double-quote template-literal sink"
    );
}

#[test]
fn ignores_template_xss_static_string() {
    let prop: String = ['i', 'n', 'n', 'e', 'r', 'H', 'T', 'M', 'L']
        .iter()
        .collect();
    let code = format!("root.{prop} = \"<p>safe static</p>\";");
    // Static double-quoted strings without template-literal interpolation must NOT trigger.
    assert!(
        check("src/C.astro", &code).is_none(),
        "static string assignment should not flag"
    );
}

#[test]
fn ignores_template_without_interpolation() {
    let prop: String = ['i', 'n', 'n', 'e', 'r', 'H', 'T', 'M', 'L']
        .iter()
        .collect();
    let code = format!("root.{prop} = `<p>safe template</p>`;");
    // Template literal without ${} interpolation is safe.
    assert!(
        check("src/C.astro", &code).is_none(),
        "template literal without interp should not flag"
    );
}

#[test]
fn skips_non_frontend() {
    assert!(check("src/main.rs", "bg-[${ x }]").is_none());
}

#[test]
fn skips_tests() {
    assert!(check("src/tests/C.tsx", "bg-[${ x }]").is_none());
}
