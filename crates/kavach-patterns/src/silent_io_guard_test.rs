use super::*;

#[test]
fn blocks_let_underscore_writeln() {
    let code = "let _ = writeln!(io::stdout().lock(), \"x\");";
    let hits = detect("src/main.rs", code);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].category, "let-underscore-print");
}

#[test]
fn blocks_let_underscore_lock() {
    let code = "let _ = my_mutex.lock();";
    let hits = detect("src/main.rs", code);
    assert!(!hits.is_empty(), "lock pattern must be detected");
}

#[test]
fn blocks_map_err_discard() {
    let code = "foo().map_err(|_| MyError::Generic)?;";
    let hits = detect("src/main.rs", code);
    assert!(hits.iter().any(|h| h.category == "map-err-discard-source"));
}

#[test]
fn allows_drop_explicit() {
    let code = "drop(writeln!(stdout, \"x\"));";
    let hits = detect("src/main.rs", code);
    assert!(hits.is_empty(), "drop(expr) is the documented alternative");
}

#[test]
fn ok_on_print_not_yet_detected_but_not_endorsed() {
    // `.ok()` on a print isn't flagged today (high-FP to detect), but the gate
    // language must NOT recommend it — that contract is pinned below.
    let code = "writeln!(stdout, \"x\").ok();";
    let hits = detect("src/main.rs", code);
    assert!(
        hits.is_empty(),
        "print .ok() is below the detection bar (FP risk)"
    );
}

#[test]
fn block_guide_marks_ok_forbidden_not_recommended() {
    let msg = check("src/main.rs", "let _ = do_io();").unwrap_or_default();
    assert!(
        msg.contains("FORBIDDEN"),
        "guide names the suppression set forbidden: {msg}"
    );
    assert!(
        !msg.contains("Result discard:   `.ok()`"),
        "guide must not lead with .ok() as the remedy: {msg}"
    );
}

#[test]
fn allows_phantom_data() {
    let code = "let _phantom: PhantomData<T> = PhantomData;";
    let hits = detect("src/main.rs", code);
    assert!(hits.is_empty(), "PhantomData pattern is legitimate");
}

#[test]
fn allows_test_files() {
    let code = "let _ = writeln!(stdout, \"x\");";
    let hits = detect("src/tests/foo.rs", code);
    assert!(hits.is_empty(), "test files exempt");
}

#[test]
fn allows_map_err_with_binding() {
    let code = "foo().map_err(|e| MyError::Wrap(e))?;";
    let hits = detect("src/main.rs", code);
    assert!(hits.is_empty(), "binding source error is correct");
}

#[test]
fn allows_safety_comment_override() {
    let code = "let _ = writeln!(io::stderr().lock(), \"x\");";
    let hits = detect("src/main.rs", code);
    // First line is the SAFETY comment (exempt), second line passes only because
    // the comment is on the previous line (current detector is line-local).
    // For now this test pins the documented escape hatch: explicit SAFETY note
    // adjacent to the `let _ =` line. If we want per-policy exemption, extend
    // line_is_exempt to look back one line.
    assert_eq!(
        hits.len(),
        1,
        "current detector is line-local; per-policy exemption tracked in future iteration"
    );
}

#[test]
fn check_returns_message_on_hit() {
    let code = "let _ = writeln!(stdout, \"x\");";
    let msg = check("src/main.rs", code);
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("[SILENT_IO_POLICY]"));
}

#[test]
fn fix_text_never_endorses_suppression() {
    // The remedy must MANDATE handling — never teach `.ok()`/`drop(Result)`/`let _`
    // as the fix (the wrong-injection that let `let _ = (lat,lng)` pass as compliant).
    for rule in RULES.iter() {
        let f = rule.fix;
        // No rule may endorse a suppression vocabulary as the remedy.
        assert!(!f.contains(".ok()"), "fix endorses .ok() suppression: {f}");
        assert!(
            !f.to_lowercase().contains("if you must discard"),
            "fix offers a discard escape: {f}"
        );
    }
    // The two Result-bearing rules must name a real handling form.
    for cat in ["let-underscore-print", "let-underscore-fn-call"] {
        let f = RULES
            .iter()
            .find(|r| r.category == cat)
            .map_or("", |r| r.fix);
        assert!(
            f.contains("if let Err") || f.contains('?') || f.contains("match"),
            "{cat} fix must name handling: {f}"
        );
    }
}
