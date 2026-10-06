//! AST and source traversal engine using `ignore` and `ast-grep-core`.

pub mod scan;
pub mod walker;

pub use scan::{scan_ast, AstMatch, SupportLang};
pub use walker::{walk_source_files, WalkConfig};
