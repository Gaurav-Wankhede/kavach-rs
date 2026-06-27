//! Value-shape signal: does the candidate's captured RHS match the role's value_regex.

#[must_use]
pub(super) fn score(value_regex: Option<&str>, value: Option<&str>) -> f32 {
    let (Some(pat), Some(val)) = (value_regex, value) else {
        return 0.0;
    };
    match regex::Regex::new(pat) {
        Ok(re) if re.is_match(val) => 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
#[path = "value_test.rs"]
mod value_test;
