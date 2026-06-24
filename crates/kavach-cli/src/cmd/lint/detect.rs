// Stack detection by manifest presence — language-agnostic selector for the
// strict-lint profile. SOURCE: decision.lint.language-profile-template.
use std::path::Path;

/// A detected project stack and the strict-rules artifact it installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stack {
    Rust,
    Ts,
    Go,
}

impl Stack {
    /// The manifest a strict profile targets (`Cargo.toml` / `tsconfig.json` /
    /// `.golangci.yml`) — the file `init` writes or audits.
    pub(crate) const fn manifest(self) -> &'static str {
        match self {
            Self::Rust => "Cargo.toml",
            Self::Ts => "tsconfig.json",
            Self::Go => ".golangci.yml",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Ts => "TypeScript/JS",
            Self::Go => "Go",
        }
    }
}

/// Detect every stack present at `root` by its project manifest (a polyglot repo
/// returns more than one). Empty = no recognized stack (init is a no-op then).
pub(crate) fn detect(root: &Path) -> Vec<Stack> {
    let mut found = Vec::new();
    if root.join("Cargo.toml").is_file() {
        found.push(Stack::Rust);
    }
    if root.join("package.json").is_file() || root.join("tsconfig.json").is_file() {
        found.push(Stack::Ts);
    }
    if root.join("go.mod").is_file() {
        found.push(Stack::Go);
    }
    found
}
