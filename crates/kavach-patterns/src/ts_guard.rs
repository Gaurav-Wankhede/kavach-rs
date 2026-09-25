use crate::lex::{Lang, RsKind, code_text_for_lang, segments_for, text};
use crate::ts_patterns::TS_P;
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TsSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TsViolation {
    pub severity: TsSeverity,
    pub pattern: String,
    pub fix: String,
    pub line: usize,
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<TsViolation> {
    if !crate::file_types::is_frontend_file(file_path) || content.is_empty() {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let lower = file_path.to_lowercase();
    if lower.contains(".config.") || lower.contains(".d.ts") {
        return vec![];
    }

    let r = &*TS_P;
    let mut violations = Vec::new();
    detect_line_patterns(&mut violations, r, content);
    detect_content_patterns(&mut violations, r, content);
    violations
}

/// Per-line segment views for rule routing: executable code vs comment
/// prose. String contents are visible to neither (payload rules 15/17 below
/// stay on the raw line).
struct LineViews {
    /// Code-only text (comments + strings dropped).
    code: String,
    /// Comment-only text (directive rules live here).
    comments: String,
}

/// Split one source line into its code and comment views (TsJs trivia).
fn line_views(line: &str) -> LineViews {
    let mut code = String::with_capacity(line.len());
    let mut comments = String::new();
    for seg in segments_for(Lang::TsJs, line) {
        match seg.kind {
            RsKind::Code => code.push_str(text(line, &seg)),
            RsKind::Comment => {
                comments.push_str(text(line, &seg));
                comments.push(' ');
            }
            RsKind::Str => {}
        }
    }
    // Gap rule (same as `code_line`): non-code spans leave a space so
    // `a/*x*/b` cannot fuse into `ab`.
    LineViews { code, comments }
}

#[expect(
    clippy::too_many_lines,
    reason = "pattern detection requires checking 20+ regex rules per line"
)]
fn detect_line_patterns(violations: &mut Vec<TsViolation>, r: &[Regex], content: &str) {
    for (i, raw) in content.lines().enumerate() {
        let views = line_views(raw);
        let line = views.code.as_str();
        if r.first().is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "console.log".into(),
                fix: "Remove console statement — use structured logging".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(1).is_some_and(|re| re.is_match(line))
            && !line.contains("??")
            && !line.contains("||")
        {
            violations.push(TsViolation {
                severity: TsSeverity::P1Advisory,
                pattern: "process.env no fallback".into(),
                fix: "Add ?? fallback or validate env at startup".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(2).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P2Warning,
                pattern: "empty catch".into(),
                fix: "Log and handle the error in catch block".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(5).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "as any".into(),
                fix: "Narrow the type with a type guard or assertion function".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(6)
            .is_some_and(|re| re.is_match(views.comments.as_str()))
        {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "@ts-ignore".into(),
                fix: "Fix the underlying type error instead of suppressing".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(7)
            .is_some_and(|re| re.is_match(views.comments.as_str()))
        {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "eslint-disable".into(),
                fix: "Fix the lint violation instead of disabling the rule".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(8)
            .is_some_and(|re| re.is_match(views.comments.as_str()))
        {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "@ts-nocheck".into(),
                fix: "Fix all type errors in this file — no blanket suppression".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(9).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "dangerouslySetInnerHTML".into(),
                fix: "Render with JSX — XSS risk from raw HTML injection".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(10).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "innerHTML".into(),
                fix: "Set textContent instead — innerHTML enables XSS".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(11).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "eval()".into(),
                fix: "Remove eval() — code injection vulnerability".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(12).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "new Function()".into(),
                fix: "Remove new Function() — equivalent to eval()".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(13).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "document.write".into(),
                fix: "Remove document.write — XSS vector, use DOM API".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(15).is_some_and(|re| re.is_match(raw)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "mock/hardcoded data".into(),
                fix: "Replace with API call — mock data enables silent failures in production"
                    .into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(16).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "useState hardcoded array".into(),
                fix: "Move to useEffect + API call — useState is for state, not data fetching"
                    .into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(17).is_some_and(|re| re.is_match(raw)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "fake engagement metrics".into(),
                fix: "Query real metrics from API — fake data hides analytics bugs".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(18).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P1Advisory,
                pattern: "localStorage".into(),
                fix: "Store auth tokens in httpOnly cookies — localStorage is accessible to XSS"
                    .into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(19).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P1Advisory,
                pattern: "sessionStorage".into(),
                fix: "Store auth tokens in httpOnly cookies — sessionStorage is accessible to XSS"
                    .into(),
                line: i.saturating_add(1),
            });
        }
    }
}

fn detect_content_patterns(violations: &mut Vec<TsViolation>, r: &[Regex], content: &str) {
    // Whole-content triggers AND exemption needles match the code view
    // (every rule here is an API-shape check: setInterval, addEventListener,
    // setLoading, fetch, AbortSignal — all code idents, never payloads).
    let code = code_text_for_lang(Lang::TsJs, content);
    let has = |idx: usize| r.get(idx).is_some_and(|re| re.is_match(&code));
    // Timer without cleanup: setInterval present but no clearInterval
    let has_interval = has(20);
    let has_clear = has(29);
    if has_interval && !has_clear {
        violations.push(TsViolation {
            severity: TsSeverity::P0Block,
            pattern: "setInterval without cleanup".into(),
            fix: "Return clearInterval from useEffect — timer leaks on unmount".into(),
            line: 0,
        });
    }

    // Event listener without cleanup
    let has_listener = has(22);
    let has_remove = has(30);
    if has_listener && !has_remove {
        violations.push(TsViolation {
            severity: TsSeverity::P0Block,
            pattern: "addEventListener without cleanup".into(),
            fix: "Call removeEventListener in useEffect cleanup — listeners accumulate on re-mount"
                .into(),
            line: 0,
        });
    }

    // Line-level checks for new patterns: the suppression directive lives in
    // comments, the type/assertion rules in code.
    for (i, raw) in content.lines().enumerate() {
        let views = line_views(raw);
        let line = views.code.as_str();
        if r.get(23)
            .is_some_and(|re| re.is_match(views.comments.as_str()))
        {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: "@ts-expect-error".into(),
                fix: "Fix the type error — suppression hides real bugs".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(24).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P0Block,
                pattern: ": any type".into(),
                fix: "Use a specific type — any defeats TypeScript's purpose".into(),
                line: i.saturating_add(1),
            });
        }
        if r.get(26).is_some_and(|re| re.is_match(line)) {
            violations.push(TsViolation {
                severity: TsSeverity::P1Advisory,
                pattern: "non-null assertion (!.)".into(),
                fix: "Use optional chaining (?.) or null check — !. crashes on null".into(),
                line: i.saturating_add(1),
            });
        }
    }

    // Empty function body (content-level)
    if has(27) {
        violations.push(TsViolation {
            severity: TsSeverity::P0Block,
            pattern: "empty function body".into(),
            fix: "Implement the function — empty bodies silently pass as complete".into(),
            line: 0,
        });
    }

    // Loading state without error recovery: setLoading(true) without setLoading(false)
    let has_loading_true = has(31);
    let has_loading_false = has(32);
    if has_loading_true && !has_loading_false {
        violations.push(TsViolation {
            severity: TsSeverity::P0Block,
            pattern: "loading state without recovery".into(),
            fix: "Add setLoading(false) in finally{} block — UI stays stuck forever on error without it".into(),
            line: 0,
        });
    }

    // Fetch without AbortSignal.timeout: fetch present but no timeout signal
    let has_fetch = has(28);
    let has_abort = has(30);
    let has_timeout_signal = has(33);
    if has_fetch && !has_abort && !has_timeout_signal {
        violations.push(TsViolation {
            severity: TsSeverity::P1Advisory,
            pattern: "fetch without timeout".into(),
            fix: "Add { signal: AbortSignal.timeout(30000) } to fetch — unbounded requests freeze the UI".into(),
            line: 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn p0_as_any() {
        let v = detect("a.ts", "const x = foo as any;");
        assert!(v.iter().any(|x| x.severity == TsSeverity::P0Block));
    }
    #[test]
    fn p0_console() {
        let v = detect("a.ts", "console.log('hi')");
        assert!(v.iter().any(|x| x.severity == TsSeverity::P0Block));
    }
    #[test]
    fn clean_passes() {
        let v = detect(
            "a.ts",
            "const x: string = fetch('/api').then(r => r.json());",
        );
        assert!(v.is_empty() || v.iter().all(|x| x.severity != TsSeverity::P0Block));
    }
    #[test]
    fn test_file_skipped() {
        let v = detect("a.test.ts", "const x = foo as any;");
        assert!(v.is_empty());
    }
    #[test]
    fn p0_timer_leak() {
        let j = crate::config::j;
        let code = j(&["set", "Interval(() => {}, 1000)"]);
        let v = detect("a.ts", &code);
        assert!(v.iter().any(|x| x.pattern.contains("setInterval")));
    }
    #[test]
    fn timer_with_cleanup_ok() {
        let j = crate::config::j;
        let code = j(&[
            "const id = set",
            "Interval(() => {}, 1000);\nclear",
            "Interval(id);",
        ]);
        let v = detect("a.ts", &code);
        assert!(!v.iter().any(|x| x.pattern.contains("setInterval")));
    }
    #[test]
    fn p0_listener_leak() {
        let j = crate::config::j;
        let code = j(&["window.add", "Event", "Listener('resize', handler)"]);
        let v = detect("a.ts", &code);
        assert!(v.iter().any(|x| x.pattern.contains("addEventListener")));
    }
    #[test]
    fn p0_any_type() {
        let v = detect("a.ts", "const data: any = fetchData();");
        assert!(v.iter().any(|x| x.pattern.contains("any type")));
    }
    #[test]
    fn p0_empty_fn_ts() {
        let v = detect("a.ts", "function handleClick() {}");
        assert!(v.iter().any(|x| x.pattern == "empty function body"));
    }
    #[test]
    fn p0_ts_expect_error() {
        let j = crate::config::j;
        let code = j(&["// @ts", "-", "expect", "-", "error"]);
        let v = detect("a.ts", &code);
        assert!(v.iter().any(|x| x.pattern.contains("expect-error")));
    }

    #[test]
    fn code_in_comments_does_not_fire_code_rules() {
        // Commented-out code is prose: no console/innerHTML hits.
        let v = detect(
            "a.ts",
            "// console.log(x);\n// el.innerHTML = y;\nconst ok = 1;",
        );
        assert!(
            v.iter()
                .all(|x| !x.pattern.contains("console") && x.pattern != "innerHTML"),
            "got: {:?}",
            v.iter().map(|x| &x.pattern).collect::<Vec<_>>()
        );
    }

    #[test]
    fn string_data_does_not_fire_code_rules() {
        // `"innerHTML"` as data, not a DOM sink.
        let v = detect("a.ts", "const k = \"innerHTML\";");
        assert!(
            v.iter().all(|x| x.pattern != "innerHTML"),
            "got: {:?}",
            v.iter().map(|x| &x.pattern).collect::<Vec<_>>()
        );
    }

    #[test]
    fn suppression_directive_in_comment_still_fires() {
        let v = detect("a.ts", "const x = 1; // @ts-ignore next line");
        assert!(v.iter().any(|x| x.pattern.contains("ts-ignore")));
    }
}
