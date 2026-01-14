//! This module defines the `BoxSize` type, which represents the size of a BMFF box,
//! including support for 32-bit sizes, 64-bit extended sizes, and sizes that extend
//! to the end of the file.

use core::error;
use core::fmt;

use super::error::ErrorKind;

/// Kinds of errors that can occur while processing box sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxSizeError {
    /// The specified size is too small to be valid.
    SizeTooSmall {
        /// The minimum expected size.
        expected: u64,
        /// The invalid size encountered.
        found: u64,
    },
    /// The specified size indicates an extended size, but no extended size was provided.
    ExtendedSizeMarker,
}

impl fmt::Display for BoxSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoxSizeError::SizeTooSmall { expected, found } => write!(
                f,
                "Box size is too small to be valid: expected at least {}, found {}",
                expected, found
            ),
            BoxSizeError::ExtendedSizeMarker => {
                write!(f, "Box size indicates extended size, but none was provided")
            }
        }
    }
}

impl error::Error for BoxSizeError {}

impl From<BoxSizeError> for ErrorKind {
    fn from(value: BoxSizeError) -> Self {
        match value {
            BoxSizeError::SizeTooSmall { expected: _, found } => Self::InvalidBoxSize {
                reason: "Box size is too small to be valid",
                got: found,
            },
            BoxSizeError::ExtendedSizeMarker => Self::InvalidBoxSize {
                reason: "Box size indicates extended size, but none was provided",
                got: BoxSize::MARKER_EXTENDED_SIZE as u64,
            },
        }
    }
}

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

    /// Creates a `BoxSize` from a 32-bit size field.
    ///
    /// This corresponds to reading the initial 4-byte size field from a box header.
    /// - `size = 0`: Box extends to end of file
    /// - `size = 1`: Returns error (extended size marker, use `from_u64` with largesize)
    /// - `size >= 8`: Valid compact size
    /// - `size < 8`: Returns error (too small)
    pub fn from_u32(size: u32) -> Result<Self, BoxSizeError> {
        match size {
            Self::MARKER_EOF => Ok(Self::eof()),
            Self::MARKER_EXTENDED_SIZE => Err(BoxSizeError::ExtendedSizeMarker),
            s if s < Self::SIZE_32_MIN => Err(BoxSizeError::SizeTooSmall {
                expected: Self::SIZE_32_MIN as u64,
                found: s as u64,
            }),
            s => Ok(Self(BoxSizeInner::Compact(s))),
        }
    }

    /// Creates a `BoxSize` from a 64-bit largesize field.
    ///
    /// This corresponds to reading the 8-byte largesize field when the initial
    /// size field is 1.
    /// - `size >= 16`: Valid extended size
    /// - `size < 16`: Returns error (too small for extended header)
    pub fn from_u64(size: u64) -> Result<Self, BoxSizeError> {
        match size {
            s if s < Self::SIZE_64_MIN => Err(BoxSizeError::SizeTooSmall {
                expected: Self::SIZE_64_MIN,
                found: s,
            }),
            s => Ok(Self(BoxSizeInner::Extended(s))),
        }
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

impl TryFrom<u32> for BoxSize {
    type Error = BoxSizeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_u32(value)
    }
}

impl TryFrom<u64> for BoxSize {
    type Error = BoxSizeError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::from_u64(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_size_from_u32() {
        // EOF marker
        let eof = BoxSize::from_u32(0).unwrap();
        assert!(eof.is_eof());
        assert!(!eof.is_extended());
        assert_eq!(eof.value(), None);

        // Extended size marker should error
        assert_eq!(
            BoxSize::from_u32(BoxSize::MARKER_EXTENDED_SIZE).unwrap_err(),
            BoxSizeError::ExtendedSizeMarker
        );

        // Too small
        assert_eq!(
            BoxSize::from_u32(4).unwrap_err(),
            BoxSizeError::SizeTooSmall {
                expected: 8,
                found: 4
            }
        );

        // Valid compact size
        let compact = BoxSize::from_u32(20).unwrap();
        assert!(!compact.is_eof());
        assert!(!compact.is_extended());
        assert_eq!(compact.value(), Some(20));
    }

    #[test]
    fn test_box_size_from_u64() {
        // Too small for extended size
        assert_eq!(
            BoxSize::from_u64(8).unwrap_err(),
            BoxSizeError::SizeTooSmall {
                expected: 16,
                found: 8
            }
        );

        // Valid extended size
        let extended = BoxSize::from_u64(20).unwrap();
        assert!(!extended.is_eof());
        assert!(extended.is_extended());
        assert_eq!(extended.value(), Some(20));

        // Large extended size
        let large = BoxSize::from_u64(u32::MAX as u64 + 1).unwrap();
        assert!(large.is_extended());
        assert_eq!(large.value(), Some(u32::MAX as u64 + 1));
    }

    #[test]
    fn test_box_size_equality() {
        // Compact and Extended with same value are NOT equal (different encoding)
        let compact = BoxSize::from_u32(20).unwrap();
        let extended = BoxSize::from_u64(20).unwrap();
        assert_ne!(compact, extended);

        // Same encoding with same value are equal
        let compact1 = BoxSize::from_u32(100).unwrap();
        let compact2 = BoxSize::from_u32(100).unwrap();
        assert_eq!(compact1, compact2);
    }
}
