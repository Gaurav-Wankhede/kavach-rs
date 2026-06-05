//! String-literal stripper so trigger keywords inside `"..."` don't false-fire.

/// Replace every char inside `"..."` with a space so trigger keywords in quoted
/// strings (const arrays of algorithm names, doc examples) don't false-positive.
/// Does not handle raw strings (`r"..."`) or byte strings — conservative.
pub(super) fn strip_string_literals(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_string = false;
    let mut escaped = false;
    for ch in content.chars() {
        if escaped {
            escaped = false;
            out.push(if in_string { ' ' } else { ch });
            continue;
        }
        match ch {
            '\\' if in_string => {
                escaped = true;
                out.push(' ');
            }
            '"' => {
                in_string = !in_string;
                out.push(ch);
            }
            _ if in_string => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}
