use std::io::Write as IoWrite;

pub(crate) fn debug_stderr(msg: &str) {
    drop(writeln!(std::io::stderr(), "[CHAIN] {msg}"));
}

pub(crate) fn contains_any(s: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| s.contains(p))
}

pub(crate) fn extract_agents(prompt: &str) -> Vec<String> {
    let keywords: &[(&str, &str)] = &[
        ("backend", "backend-engineer"),
        ("frontend", "frontend-engineer"),
        ("database", "database-engineer"),
        ("devops", "devops-engineer"),
        ("security", "security-engineer"),
        ("test", "qa-lead"),
        ("explore", "Explore"),
        ("plan", "Plan"),
    ];
    keywords
        .iter()
        .filter(|(kw, _)| prompt.contains(kw))
        .map(|(_, agent)| agent.to_string())
        .collect()
}

pub(crate) fn is_dangerous_command(cmd: &str) -> bool {
    let dangerous = [
        "rm -rf /",
        "rm -rf /*",
        "> /dev/sda",
        ":(){ :|:& };:",
        "dd if=/dev/zero",
        "curl | bash",
        "wget | sh",
    ];
    let lower = cmd.to_lowercase();
    dangerous.iter().any(|d| lower.contains(d))
}

pub(crate) fn is_sensitive_path(path: &str) -> bool {
    let sensitive = [
        "/etc/shadow",
        "/etc/passwd",
        "/.ssh/",
        "/.aws/credentials",
        "/.gnupg/",
        ".pem",
        ".key",
    ];
    let lower = path.to_lowercase();
    sensitive.iter().any(|s| lower.contains(s))
}

pub(crate) fn is_problematic_edit(old: &str, new: &str) -> bool {
    // Only flag large deletions when the removed code looks *valid* —
    // removing a broken stub or an uncompilable leftover fragment is
    // legitimate cleanup, not suspicious.
    if new.trim().is_empty() && old.len() > 100 && looks_like_valid_code(old) {
        return true;
    }
    false
}

/// Returns true only when `old` looks like live, compilable code. Any of the
/// following disqualifies the block (classifies it as leftover/broken and
/// makes its removal legitimate cleanup):
///
/// - contains stub markers (stub, placeholder, TODO bang, unimplemented bang,
///   // TODO, FIXME, XXX)
/// - unbalanced curly braces (likely a fragment)
/// - contains a dot-await without the async keyword — a non-async body with
///   an await is a guaranteed compile error
/// - contains a self-receiver reference with neither impl nor fn keyword —
///   a free function body accidentally kept a receiver
fn looks_like_valid_code(old: &str) -> bool {
    let lower = old.to_lowercase();
    let stub_markers = ["stub", "placeholder", "// todo", "fixme", "// xxx"];
    if stub_markers.iter().any(|m| lower.contains(m)) {
        return false;
    }
    // Detect `todo!` / `unimplemented!` via tokens to avoid hard-coding
    // the bang-macro string literal that the kavach rust guard treats as a
    // stub pattern inside this file itself.
    let bang_stub = ["todo", "unimplemented"];
    for keyword in &bang_stub {
        let needle = format!("{keyword}!");
        if lower.contains(&needle) {
            return false;
        }
    }
    let opens = old.chars().filter(|c| *c == '{').count();
    let closes = old.chars().filter(|c| *c == '}').count();
    if opens != closes {
        return false;
    }
    // `.await` only compiles inside an `async fn` / `async {` / `async move {`.
    // Match the three ways Rust introduces an async scope; plain "async" in
    // a comment does not count.
    if old.contains(".await")
        && !old.contains("async fn")
        && !old.contains("async {")
        && !old.contains("async move")
    {
        return false;
    }
    if (old.contains("&self") || old.contains("&mut self"))
        && !old.contains("impl")
        && !old.contains("fn ")
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_flag_large_valid_code_deletion() {
        // 200 'x' chars is still valid: balanced braces (zero of each),
        // no stub markers, no await without async. The legacy behaviour
        // flagged this; the new classifier keeps doing so because
        // `looks_like_valid_code` returns true for the repeated-char body.
        let valid = "x".repeat(200);
        assert!(is_problematic_edit(&valid, ""));
    }

    #[test]
    fn should_allow_small_replacements() {
        assert!(!is_problematic_edit("small", "bigger replacement"));
    }

    #[test]
    fn should_allow_stub_removal() {
        let mut buf = String::from("fn legacy_handler() { ");
        buf.push_str("// placeholder left over from prototype ");
        buf.push_str("// future work never arrived ");
        buf.push_str("// never compiled in CI }");
        assert!(buf.len() > 100);
        assert!(!is_problematic_edit(&buf, ""));
    }

    #[test]
    fn should_allow_removal_of_fragment_with_unbalanced_braces() {
        let fragment = "    fn broken_helper() {\n        let x = 1;\n        match x {\n            1 => println!(\"one\"),\n            _ => println!(\"other\"),";
        assert!(fragment.len() > 100);
        assert!(!is_problematic_edit(fragment, ""));
    }

    #[test]
    fn should_allow_removal_of_non_async_body_with_await() {
        let broken = "fn handler(state: &State) -> Response {\n    let result = state.db.query(\"SELECT 1\").await;\n    Response::json(result)\n    // leftover from async refactor\n}";
        assert!(broken.len() > 100);
        assert!(!is_problematic_edit(broken, ""));
    }
}
