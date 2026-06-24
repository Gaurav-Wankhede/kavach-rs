//! Pure path/content predicates for the nano-file guard, split out of the hub
//! to keep `nano_file_guard.rs` itself under the 100-LOC ceiling it enforces.

/// Header-region markers that exempt a file from the LOC ceiling. All must be
/// declared in the first 15 lines so they stay visible in review.
///
/// - `kavach:nano-file-exempt` — file that genuinely cannot decompose (SQL-DDL
///   `const`, generated template, big static lookup table).
/// - `// split:` / `// hub:` — the SAME intentional-split markers the sibling
///   microservice guard already honors, keeping the two guards consistent.
/// - `kavach:intentional` — a `// kavach:intentional <reason>` names a deliberate
///   ceiling + upgrade path. Minimalism is the reuse/stdlib/one-line decision, not
///   a raw LOC count, so a NAMED ceiling is intent, not bloat. SOURCE:
///   decision.harness.nano-file-ladder-not-loc.
const LOC_EXEMPT_MARKERS: [&str; 4] =
    ["kavach:nano-file-exempt", "// split:", "// hub:", "kavach:intentional"];

/// True when the file declares any exempt marker in its header region (first 15
/// lines), so it stays visible in review and cannot be buried.
#[must_use]
pub(super) fn is_loc_exempt(content: &str) -> bool {
    content
        .lines()
        .take(15)
        .any(|line| LOC_EXEMPT_MARKERS.iter().any(|m| line.contains(m)))
}

/// True when a NON-test source file carries an inline `#[cfg(test)]` module.
/// Test sidecars are exempt: a path ending `_test.rs`, `tests.rs`, or living
/// under a `/tests/` dir IS the extracted home, so it may hold the module.
#[must_use]
pub(super) fn has_inline_tests(file_path: &str, content: &str) -> bool {
    let p = file_path.replace('\\', "/");
    let is_test_sidecar = p.ends_with("_test.rs")
        || p.ends_with("/tests.rs")
        || p == "tests.rs"
        || p.contains("/tests/");
    if is_test_sidecar {
        return false;
    }
    // A `#[cfg(test)]` region is only a VIOLATION when it introduces an inline
    // module BODY (`mod tests {`), not when it declares a sibling test file
    // (`#[cfg(test)] #[path = "foo_test.rs"] mod tests;`) — the latter is the
    // PRESCRIBED fix, so blocking it is a false positive that wedges every split.
    // Walk real (non-comment) lines; for each on-its-own-line `#[cfg(test)]`, scan
    // the following few lines: a `mod <name> {` opener = inline (block); a
    // `mod <name>;` terminated declaration (with or without an intervening
    // `#[path = ...]`) = legal sidecar wiring, exempt.
    let lines: Vec<&str> = content.lines().map(str::trim_start).collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.starts_with("//") && t.starts_with("#[cfg(test)]"))
        .any(|(i, _)| gates_inline_block(lines.get(i.saturating_add(1)..).unwrap_or(&[])))
}

/// Given the lines AFTER a `#[cfg(test)]`, decide whether it gates an inline
/// module BODY. Steps over intervening attributes (`#[path=...]`), blanks, and
/// comments to the first `mod` item: a `{` opener = inline block (VIOLATION); a
/// `;` terminator = `#[path]` sidecar declaration (the PRESCRIBED fix, exempt).
/// Stops at the first unrelated statement so it never reads into code below.
fn gates_inline_block(rest: &[&str]) -> bool {
    // Find the first line that DECIDES the verdict (a `mod` item, or any
    // unrelated statement), skipping intervening attrs/blanks/comments. A `mod`
    // with `{` is an inline block; anything else (incl. `mod ...;`) is not.
    rest.iter()
        .find(|l| is_decisive(l))
        .and_then(|l| mod_tail(l))
        .is_some_and(|m| m.contains('{'))
}

/// A line is decisive (stops the forward scan) when it is the gated `mod` item
/// or any non-attribute, non-blank, non-comment statement.
fn is_decisive(line: &str) -> bool {
    mod_tail(line).is_some()
        || !(line.is_empty() || line.starts_with("#[") || line.starts_with("//"))
}

/// The text after a `mod ` keyword on a trimmed line (leading or mid-line, e.g.
/// `pub(crate) mod`), or `None` if the line declares no module.
fn mod_tail(line: &str) -> Option<&str> {
    line.strip_prefix("mod ")
        .or_else(|| line.split_once(" mod ").map(|(_, m)| m))
}

/// Directory depth below the nearest `/src/`, or `None` when the path has no
/// `src` segment. Used to enforce the depth ceiling.
#[must_use]
pub(super) fn depth_below_src(file_path: &str) -> Option<usize> {
    let marker = "/src/";
    let idx = file_path.rfind(marker)?;
    let tail = file_path.get(idx.saturating_add(marker.len())..)?;
    let last_slash = tail.rfind('/')?;
    let depth = tail
        .get(..last_slash)?
        .matches('/')
        .count()
        .saturating_add(1);
    Some(depth)
}
