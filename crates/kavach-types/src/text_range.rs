//! Zero-copy, zero-heap span identifier (`TextRange`).
//!
//! 8-byte representation storing raw byte offsets rather than 32-byte 2D line/col coordinates.
//! SOURCE: Universal High-Performance Rust Systems Engineering Handbook §Pattern 2.
use serde::{Deserialize, Serialize};

/// 8-byte byte-offset span.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[non_exhaustive]
#[repr(C)]
pub struct TextRange {
    /// Zero-indexed start byte offset.
    pub start: u32,
    /// Zero-indexed end byte offset.
    pub end: u32,
}

impl TextRange {
    /// Create a new `TextRange` from raw start and end byte offsets.
    #[inline]
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Returns the length in bytes of the span.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the range is empty (zero length).
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}
