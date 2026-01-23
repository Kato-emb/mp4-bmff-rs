//! This module defines the `BoxSize` type, which represents the size of a BMFF box,
//! including support for 32-bit sizes, 64-bit extended sizes, and sizes that extend
//! to the end of the file.

use core::fmt;

/// Internal representation of box size variants.
/// Private to enforce validation through constructors.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum BoxSizeInner {
    /// 32-bit compact size (8 <= size <= u32::MAX, size != 1)
    Compact(u32),
    /// 64-bit extended size (size >= 16)
    Extended(u64),
    /// Box extends to end of file (size field = 0)
    ToEnd,
}

/// Type-safe representation of BMFF `boxsize` values.
///
/// This type preserves the original encoding format (compact vs extended)
/// to ensure accurate round-trip serialization.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoxSize(BoxSizeInner);

impl BoxSize {
    /// The minimum size of a box with a 32-bit size field.
    const SIZE_32_MIN: u32 = 8;
    /// The minimum size of a box with a 64-bit size field.
    const SIZE_64_MIN: u64 = 16;
    /// Size field marker for 64-bit extended size.
    pub const MARKER_EXTENDED_SIZE: u32 = 1;
    /// Size field marker for "to end of file".
    pub const MARKER_EOF: u32 = 0;

    /// Creates a `BoxSize` that extends to the end of the file.
    #[inline]
    pub const fn eof() -> Self {
        Self(BoxSizeInner::ToEnd)
    }

    /// Returns `true` if the box size extends to the end of the file.
    #[inline]
    pub const fn is_eof(&self) -> bool {
        matches!(self.0, BoxSizeInner::ToEnd)
    }

    /// Returns `true` if the box size uses a 64-bit extended size field.
    #[inline]
    pub const fn is_extended(&self) -> bool {
        matches!(self.0, BoxSizeInner::Extended(_))
    }

    /// Returns the size value, or `None` if the box extends to the end of the file.
    #[inline]
    pub const fn value(&self) -> Option<u64> {
        match self.0 {
            BoxSizeInner::Compact(v) => Some(v as u64),
            BoxSizeInner::Extended(v) => Some(v),
            BoxSizeInner::ToEnd => None,
        }
    }

    pub(crate) fn new(size: u64) -> Self {
        if size == Self::MARKER_EOF as u64 {
            return Self::eof();
        }

        debug_assert!(
            size >= Self::SIZE_32_MIN as u64,
            "Box size must be at least 8 bytes (size + type fields)"
        );

        if size > u32::MAX as u64 {
            debug_assert!(
                size >= Self::SIZE_64_MIN,
                "Box size must be at least 16 bytes for extended size (size + type + largesize fields)"
            );

            Self(BoxSizeInner::Extended(size))
        } else {
            Self(BoxSizeInner::Compact(size as u32))
        }
    }

    pub(crate) fn new_compact(size: u32) -> Self {
        debug_assert!(
            size >= Self::SIZE_32_MIN,
            "Box size must be at least 8 bytes (size + type fields)"
        );

        Self(BoxSizeInner::Compact(size))
    }

    pub(crate) fn new_extended(size: u64) -> Self {
        debug_assert!(
            size >= Self::SIZE_64_MIN,
            "Box size must be at least 16 bytes for extended size (size + type + largesize fields)"
        );

        Self(BoxSizeInner::Extended(size))
    }
}

impl fmt::Debug for BoxSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            BoxSizeInner::Compact(v) => write!(f, "BoxSize::Compact({})", v),
            BoxSizeInner::Extended(v) => write!(f, "BoxSize::Extended({})", v),
            BoxSizeInner::ToEnd => write!(f, "BoxSize::ToEnd"),
        }
    }
}

impl fmt::Display for BoxSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            BoxSizeInner::Compact(v) => write!(f, "{}", v),
            BoxSizeInner::Extended(v) => write!(f, "{}", v),
            BoxSizeInner::ToEnd => write!(f, "ToEnd"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eof_returns_to_end_variant() {
        let size = BoxSize::eof();

        assert!(size.is_eof());
        assert!(!size.is_extended());
        assert_eq!(size.value(), None);
    }

    #[test]
    fn new_compact_creates_compact_variant() {
        let size = BoxSize::new_compact(100);

        assert!(!size.is_eof());
        assert!(!size.is_extended());
        assert_eq!(size.value(), Some(100));
    }

    #[test]
    fn new_extended_creates_extended_variant() {
        let size = BoxSize::new_extended(100);

        assert!(!size.is_eof());
        assert!(size.is_extended());
        assert_eq!(size.value(), Some(100));
    }

    #[test]
    fn new_with_zero_returns_eof() {
        let size = BoxSize::new(0);

        assert!(size.is_eof());
    }

    #[test]
    fn new_with_u32_range_returns_compact() {
        let size = BoxSize::new(1000);
        assert!(!size.is_extended());

        // u32::MAX should still be compact
        let max_compact = BoxSize::new(u32::MAX as u64);
        assert!(!max_compact.is_extended());
    }

    #[test]
    fn new_with_large_value_returns_extended() {
        let size = BoxSize::new(u32::MAX as u64 + 1);

        assert!(size.is_extended());
    }

    #[test]
    fn compact_and_extended_with_same_value_are_not_equal() {
        let compact = BoxSize::new_compact(100);
        let extended = BoxSize::new_extended(100);

        // Different encoding means not equal, even if numeric value is the same
        assert_ne!(compact, extended);
    }
}
