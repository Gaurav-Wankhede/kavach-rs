mod config;
mod file;
mod validation;

#[cfg(test)]
mod config_test;

#[path = "checks/file_test.rs"]
#[cfg(test)]
mod file_test;

#[path = "checks/validation_test.rs"]
#[cfg(test)]
mod validation_test;

pub use config::{classify_intent, is_valid_agent};
pub use file::{is_code_file, is_infra_file, is_large_file, is_sensitive};
pub use validation::{sanitize_path, validate_identifier};
