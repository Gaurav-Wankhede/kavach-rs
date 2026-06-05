//! Filesystem inspection wrappers, split by concern: file content (`read`) and
//! directory inspection (`inspect`).
mod inspect;
mod read;

pub use inspect::{count_lines, disk_usage, tree};
pub use read::{diff, read_file};
