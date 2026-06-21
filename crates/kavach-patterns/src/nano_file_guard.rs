// Nano-file invariants: no mod.rs · depth <=7 below src/ · new files <=100 LOC ·
// tests live in a sibling `<name>_test.rs`, never inline.
// SOURCE: https://doc.rust-lang.org/edition-guide/rust-2024/
// SOURCE: https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute

mod predicates;
mod types;

use predicates::{depth_below_src, has_inline_tests, is_loc_exempt};
pub use types::{NanoFileViolation, NanoSeverity};

pub const MAX_DEPTH_BELOW_SRC: usize = 7;
pub const MAX_LOC_NEW_FILE: usize = 100;

#[must_use]
pub fn detect(file_path: &str, content: &str, tool_name: &str) -> Vec<NanoFileViolation> {
    let mut v = Vec::new();
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return v;
    }

    if file_path.ends_with("/mod.rs") || file_path.ends_with("\\mod.rs") {
        v.push(NanoFileViolation {
            severity: NanoSeverity::P0Block,
            pattern: "legacy mod.rs file",
            fix: "Rust 2024 forbids mod.rs. Use foo.rs + foo/ pattern: \
                  rename mod.rs to <parent_module>.rs."
                .to_owned(),
        });
    }

    if let Some(depth) = depth_below_src(file_path)
        && depth > MAX_DEPTH_BELOW_SRC
    {
        v.push(NanoFileViolation {
            severity: NanoSeverity::P0Block,
            pattern: "directory depth exceeds 7",
            fix: format!(
                "depth={depth} below src exceeds {MAX_DEPTH_BELOW_SRC}. \
                 Flatten the path or extract to a sibling crate."
            ),
        });
    }

    if has_inline_tests(file_path, content) {
        v.push(NanoFileViolation {
            severity: NanoSeverity::P0Block,
            pattern: "inline test module",
            fix: "tests must live in a sibling `<name>_test.rs`, never inline. Move \
                  the `#[cfg(test)] mod tests { … }` block out: for `foo.rs` create \
                  `foo_test.rs` and declare \
                  `#[cfg(test)] #[path = \"foo_test.rs\"] mod tests;`."
                .to_owned(),
        });
    }

    let loc = content.lines().count();
    if loc > MAX_LOC_NEW_FILE && !is_loc_exempt(content) {
        // Write (new) and Edit (existing) both HARD-BLOCK over budget: an edit that
        // pushes a file past 100 lines must split into the same deep hub+leaf
        // hierarchy (foo.rs + foo/bar.rs), smallest reusable files, no duplication,
        // mod.rs forbidden — identical discipline to a new file.
        let is_new = tool_name == "Write";
        v.push(NanoFileViolation {
            severity: NanoSeverity::P0Block,
            pattern: if is_new {
                "new file exceeds 100 LOC"
            } else {
                "file exceeds 100 LOC"
            },
            fix: format!(
                "lines={loc} exceeds {MAX_LOC_NEW_FILE}. Split into a deep hub+leaf \
                 hierarchy (foo.rs + foo/bar.rs + foo/baz.rs): smallest reusable files, \
                 NO duplication, NO mod.rs. Reuse existing modules — check first with \
                 `rg`/`fd`/`ast-grep` before adding code. Recurse: a leaf >100 splits again."
            ),
        });
    }

    v
}

#[cfg(test)]
#[path = "nano_file_guard/tests.rs"]
mod tests;
