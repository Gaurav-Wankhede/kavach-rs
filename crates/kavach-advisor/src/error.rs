use thiserror::Error;

/// Errors produced by the advisor HTTP client.
///
/// Recovery strategy:
/// - [`AdvisorError::MissingApiKey`] — permanent, user must set the env var
/// - [`AdvisorError::ApiKeyNotUnicode`] — permanent, user has non-UTF-8 bytes in env value
/// - [`AdvisorError::Http`] — potentially transient, caller may retry
/// - [`AdvisorError::Io`] — body read failure, treat as transient
/// - [`AdvisorError::NoTextBlock`] — permanent API contract violation
///
/// Per `std::env::VarError` docs, the std error type loses the variable name
/// context — see <https://doc.rust-lang.org/std/env/enum.VarError.html>. We
/// split into two variants and carry the offending `OsString` so operators see
/// exactly which key is broken and how (`NotPresent` vs `NotUnicode`).
#[derive(Debug, Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "constructed and matched cross-crate; non_exhaustive would break E0639"
)]
pub enum AdvisorError {
    #[error("ANTHROPIC_API_KEY environment variable is not set")]
    MissingApiKey,

    #[error("ANTHROPIC_API_KEY environment variable contains non-UTF-8 bytes: {0:?}")]
    ApiKeyNotUnicode(std::ffi::OsString),

    #[error("Anthropic API request failed: {0}")]
    Http(#[from] ureq::Error),

    #[error("Failed to read response body: {0}")]
    Io(#[from] std::io::Error),

    #[error("API response contained no text block")]
    NoTextBlock,
}

impl AdvisorError {
    /// Returns `true` for errors that may succeed on retry.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Http(_) | Self::Io(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_retryable_when_io_error() {
        let err = AdvisorError::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
        assert!(err.is_retryable());
    }

    #[test]
    fn should_not_be_retryable_when_missing_api_key() {
        assert!(!AdvisorError::MissingApiKey.is_retryable());
    }

    #[test]
    fn should_not_be_retryable_when_no_text_block() {
        assert!(!AdvisorError::NoTextBlock.is_retryable());
    }
}
