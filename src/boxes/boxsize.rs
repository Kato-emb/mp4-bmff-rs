//! This module defines the `BoxSize` enum, which represents the size of a BMFF box,
//! including support for 32-bit sizes, 64-bit sizes, and sizes that extend to
//! the end of the file.

use core::error;
use core::fmt;

use super::error::ErrorKind;

/// This module defines the `BoxSize` enum, which represents the size of a BMFF box,
/// including support for 32-bit sizes, 64-bit sizes, and sizes that extend to
/// the end of the file.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoxSize {
    /// 32-bit box size.
    Size32(u32),
    /// 64-bit box size.
    Size64(u64),
    /// Box extends to the end of the file.
    ToEnd,
}

impl BoxSize {
    /// The minimum size of a box with a 32-bit size field.
    const SIZE_32_MIN: u32 = 8;
    /// The minimum size of a box with a 64-bit size field.
    const SIZE_64_MIN: u64 = 16;
    /// Size field for 64-bit extended size.
    const MARKER_EXTENDED_SIZE: u32 = 1;
    /// Size field for "to end of file".
    const MARKER_EOF: u32 = 0;

    /// Returns the size as a u64, or None if the size extends to the end of the file.
    pub const fn as_u64(&self) -> Option<u64> {
        match self {
            BoxSize::Size32(s) => Some(*s as u64),
            BoxSize::Size64(s) => Some(*s),
            BoxSize::ToEnd => None,
        }
    }

    /// Creates a `BoxSize` from a 32-bit size field.
    pub fn from_u32(size: u32) -> Result<Self, BoxSizeError> {
        match size {
            Self::MARKER_EOF => Ok(BoxSize::ToEnd),
            Self::MARKER_EXTENDED_SIZE => Err(BoxSizeError::ExtendedSizeMarker),
            s if s < Self::SIZE_32_MIN => Err(BoxSizeError::SizeTooSmall),
            s => Ok(BoxSize::Size32(s)),
        }
    }

    /// Creates a `BoxSize` from a 64-bit size field.
    pub fn from_u64(size: u64) -> Result<Self, BoxSizeError> {
        match size {
            s if s < Self::SIZE_64_MIN => Err(BoxSizeError::SizeTooSmall),
            s if s > u32::MAX as u64 => Ok(BoxSize::Size64(s)),
            s => Ok(BoxSize::Size32(s as u32)),
        }
    }
}

impl fmt::Debug for BoxSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoxSize::Size32(s) => write!(f, "BoxSize::Size32({})", s),
            BoxSize::Size64(s) => write!(f, "BoxSize::Size64({})", s),
            BoxSize::ToEnd => write!(f, "BoxSize::ToEnd"),
        }
    }
}

impl fmt::Display for BoxSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoxSize::Size32(s) => write!(f, "{}", s),
            BoxSize::Size64(s) => write!(f, "{}", s),
            BoxSize::ToEnd => write!(f, "to end of file"),
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

/// Kinds of errors that can occur while processing box sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxSizeError {
    /// The specified size is too small to be valid.
    SizeTooSmall,
    /// The specified size causes an overflow.
    SizeOverflow,
    /// The specified size indicates an extended size, but no extended size was provided.
    ExtendedSizeMarker,
}

impl fmt::Display for BoxSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoxSizeError::SizeTooSmall => write!(f, "Box size is too small to be valid"),
            BoxSizeError::SizeOverflow => write!(f, "Box size causes an overflow"),
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
            BoxSizeError::SizeTooSmall => Self::InvalidBoxSize,
            BoxSizeError::SizeOverflow => Self::InvalidBoxSize,
            BoxSizeError::ExtendedSizeMarker => Self::InvalidBoxSize,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_size_from_u32() {
        assert_eq!(BoxSize::from_u32(0).unwrap(), BoxSize::ToEnd);
        assert_eq!(
            BoxSize::from_u32(BoxSize::MARKER_EXTENDED_SIZE).unwrap_err(),
            BoxSizeError::ExtendedSizeMarker
        );
        assert_eq!(
            BoxSize::from_u32(4).unwrap_err(),
            BoxSizeError::SizeTooSmall
        );
        assert_eq!(BoxSize::from_u32(20).unwrap(), BoxSize::Size32(20));
    }

    #[test]
    fn test_box_size_from_u64() {
        assert_eq!(
            BoxSize::from_u64(8).unwrap_err(),
            BoxSizeError::SizeTooSmall
        );
        assert_eq!(BoxSize::from_u64(20).unwrap(), BoxSize::Size32(20));
        assert_eq!(
            BoxSize::from_u64(u32::MAX as u64 + 1).unwrap(),
            BoxSize::Size64(u32::MAX as u64 + 1)
        );
    }

    #[test]
    fn test_box_size_as_u64() {
        assert_eq!(BoxSize::Size32(20).as_u64(), Some(20));
        assert_eq!(BoxSize::Size64(1_000_000_000).as_u64(), Some(1_000_000_000));
        assert_eq!(BoxSize::ToEnd.as_u64(), None);
    }
}
