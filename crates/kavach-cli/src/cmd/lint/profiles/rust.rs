// Canonical strict Rust lint profile — verbatim from kavach-rs Cargo.toml:105-230
// (the reference body). `kavach lint init` appends this to a project's Cargo.toml
// when none is present. SOURCE: decision.lint.language-profile-template.

/// The `[workspace.lints]` table a strict Rust project installs. Each crate then
/// opts in with `[lints] workspace = true`. No suppression: prefer `#[expect(reason=)]`.
pub(crate) const RUST_LINTS: &str = r#"[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "deny"
non_ascii_idents = "forbid"
dead_code = "deny"
unused_imports = "deny"
unused_variables = "deny"
unreachable_pub = "deny"
trivial_casts = "deny"
trivial_numeric_casts = "deny"
missing_debug_implementations = "deny"
unused_lifetimes = "deny"
unused_qualifications = "deny"
elided_lifetimes_in_paths = "deny"
explicit_outlives_requirements = "deny"
future_incompatible = { level = "deny", priority = -1 }
nonstandard_style = { level = "deny", priority = -1 }
rust_2018_idioms = { level = "deny", priority = -1 }
unused = { level = "deny", priority = -1 }

[workspace.lints.clippy]
correctness = { level = "deny", priority = -1 }
suspicious = { level = "deny", priority = -1 }
perf = { level = "deny", priority = -1 }
style = { level = "deny", priority = -1 }
complexity = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
exit = "deny"
indexing_slicing = "deny"
arithmetic_side_effects = "deny"
allow_attributes = "deny"
allow_attributes_without_reason = "deny"
let_underscore_must_use = "deny"

# Each crate then opts in with its own `[lints]` table:
#   [lints]
#   workspace = true
"#;
