//! Error types for BMFF box operations.
//!
//! This module provides error handling types for parsing and encoding BMFF boxes.
//! The error system is designed to provide detailed context about what went wrong
//! and where the error occurred.
//!
//! # Error Types
//!
//! - [`Error`]: The main error type containing kind, location, and optional source
//! - [`ErrorKind`]: An enum describing the specific type of error
//! - [`Result<T>`]: A type alias for `Result<T, Error>`
//!
//! # Error Context
//!
//! Errors can include optional context:
//! - **Offset**: The byte position where the error occurred
//! - **Box type**: The box being processed when the error occurred
//! - **Source**: An underlying error (with `std` feature)
//!
//! # Example
//!
//! ```
//! use mp4_bmff::error::{Error, ErrorKind, Result};
//!
//! fn parse_data(data: &[u8]) -> Result<u32> {
//!     if data.len() < 4 {
//!         return Err(Error::new(ErrorKind::NotEnoughBytes {
//!             expected: 4,
//!             remaining: data.len(),
//!         }));
//!     }
//!     Ok(u32::from_be_bytes([data[0], data[1], data[2], data[3]]))
//! }
//!
//! let result = parse_data(&[0x00, 0x01]);
//! assert!(result.is_err());
//! let err = result.unwrap_err();
//! assert!(matches!(err.kind(), ErrorKind::NotEnoughBytes { .. }));
//! ```

use core::error;
use core::fmt;

use crate::types::FourCC;

use crate::BoxType;

use crate::cursor::Error as CursorError;
use crate::cursor::ErrorKind as CursorErrorKind;

/// Result type for BMFF box operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Classification of errors that can occur while processing BMFF boxes.
///
/// This enum categorizes all possible error conditions when parsing or
/// encoding BMFF data. Each variant provides specific context about
/// what went wrong.
///
/// # Categories
///
/// - **Buffer errors**: `NotEnoughBytes`, `BufferTooLarge`, `Overflow`
/// - **Box structure errors**: `MismatchedBoxSize`, `MismatchedBoxType`, `InvalidBoxSize`
/// - **Box content errors**: `InvalidBoxVersion`, `InvalidBoxFlags`, `InvalidBoxField`
/// - **Container errors**: `BoxMissing`, `BoxDuplicate`
/// - **I/O errors**: `Io` (with `std` feature)
/// - **Other**: `Other` for miscellaneous errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// An integer overflow occurred during size calculation.
    Overflow,

    /// Insufficient bytes available for the requested operation.
    ///
    /// This is the most common error when parsing truncated data.
    NotEnoughBytes {
        /// Number of bytes required.
        expected: usize,
        /// Number of bytes actually available.
        remaining: usize,
    },

    /// The provided buffer exceeds the maximum allowed size.
    BufferTooLarge {
        /// Size of the provided buffer.
        expected: u64,
        /// Maximum allowed size.
        max: u64,
    },

    /// Box size in header doesn't match actual content size.
    MismatchedBoxSize {
        /// Size declared in the box header.
        expected: u64,
        /// Actual size of the box content.
        found: u64,
    },

    /// Box type in header doesn't match expected type.
    MismatchedBoxType {
        /// Expected box type.
        expected: BoxType,
        /// Actual box type found.
        found: BoxType,
    },

    /// Box size value is invalid or malformed.
    InvalidBoxSize {
        /// Description of why the size is invalid.
        reason: &'static str,
        /// The invalid size value.
        got: u64,
    },

    /// Box type value is invalid or unsupported.
    InvalidBoxType {
        /// Description of why the type is invalid.
        reason: &'static str,
        /// The invalid type FourCC.
        got: FourCC,
    },

    /// Box version is not supported or invalid for this box type.
    InvalidBoxVersion {
        /// Description of why the version is invalid.
        reason: &'static str,
        /// The invalid version number.
        got: u8,
    },

    /// Box flags contain invalid or unsupported values.
    InvalidBoxFlags {
        /// Description of why the flags are invalid.
        reason: &'static str,
        /// The invalid flags value (24-bit).
        got: u32,
    },

    /// A box field contains an invalid value.
    InvalidBoxField {
        /// Name of the invalid field.
        field: &'static str,
        /// Description of why the value is invalid.
        reason: &'static str,
    },

    /// A required child box is missing from a container box.
    BoxMissing {
        /// Type of the required box.
        required: BoxType,
    },

    /// A box that should appear at most once was found multiple times.
    BoxDuplicate {
        /// Type of the duplicate box.
        duplicate: BoxType,
    },

    /// An I/O error occurred during reading or writing.
    ///
    /// Only available with the `std` feature.
    #[cfg(feature = "std")]
    Io,

    /// An error that doesn't fit other categories.
    Other {
        /// Description of the error.
        description: &'static str,
    },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Overflow => write!(f, "integer overflow"),
            ErrorKind::NotEnoughBytes {
                expected,
                remaining,
            } => write!(
                f,
                "not enough bytes: expected {expected}, but only {remaining} remaining"
            ),
            ErrorKind::BufferTooLarge { expected, max } => {
                write!(
                    f,
                    "buffer too large: expected {expected}, maximum allowed is {max}"
                )
            }
            ErrorKind::MismatchedBoxSize { expected, found } => {
                write!(f, "mismatched box size: expected {expected}, found {found}")
            }
            ErrorKind::MismatchedBoxType { expected, found } => {
                write!(
                    f,
                    "mismatched box type: expected '{expected}', found '{found}'"
                )
            }
            ErrorKind::InvalidBoxSize { reason, got } => {
                write!(f, "invalid box size: {reason} (got {got})")
            }
            ErrorKind::InvalidBoxType { reason, got } => {
                write!(f, "invalid box type: {reason} (got {got})")
            }
            ErrorKind::InvalidBoxVersion { reason, got } => {
                write!(f, "invalid box version: {reason} (got {got})")
            }
            ErrorKind::InvalidBoxFlags { reason, got } => {
                write!(f, "invalid box flags: {reason} (got {got})")
            }
            ErrorKind::InvalidBoxField { field, reason } => {
                write!(f, "invalid box field '{field}': {reason}")
            }
            ErrorKind::BoxMissing { required } => {
                write!(f, "required box '{required}' is missing")
            }
            ErrorKind::BoxDuplicate { duplicate } => {
                write!(f, "duplicate box '{duplicate}' found")
            }
            #[cfg(feature = "std")]
            ErrorKind::Io => write!(f, "I/O error"),
            ErrorKind::Other { description } => write!(f, "error: {description}"),
        }
    }
}

/// An error that occurred while processing a BMFF box.
///
/// This error type combines an [`ErrorKind`] with optional context about
/// where the error occurred (byte offset and/or box type). This information
/// helps diagnose issues in malformed or corrupted BMFF files.
///
/// # Context Methods
///
/// Use builder methods to add context to errors:
/// - [`with_offset`](Self::with_offset): Add byte position
/// - [`with_box_type`](Self::with_box_type): Add box type context
///
/// # Example
///
/// ```
/// use mp4_bmff::error::{Error, ErrorKind};
/// use mp4_bmff::BoxType;
///
/// let err = Error::new(ErrorKind::InvalidBoxVersion {
///     reason: "version must be 0 or 1",
///     got: 2,
/// })
/// .with_box_type(BoxType::MVHD)
/// .with_offset(100);
///
/// assert_eq!(err.kind(), ErrorKind::InvalidBoxVersion {
///     reason: "version must be 0 or 1",
///     got: 2,
/// });
/// assert_eq!(err.offset(), Some(100));
/// assert_eq!(err.box_type(), Some(BoxType::MVHD));
/// ```
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    offset: Option<u64>,
    box_type: Option<BoxType>,
    #[cfg(feature = "std")]
    source: Option<Box<dyn error::Error + Send + Sync + 'static>>,
}

impl Error {
    /// Creates a new error with the given kind.
    ///
    /// The error will have no offset or box type context. Use
    /// [`with_offset`](Self::with_offset) and [`with_box_type`](Self::with_box_type)
    /// to add context.
    pub const fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            offset: None,
            box_type: None,
            #[cfg(feature = "std")]
            source: None,
        }
    }

    /// Creates an error with the given kind and byte offset.
    pub(crate) const fn at(kind: ErrorKind, offset: u64) -> Self {
        Self {
            kind,
            offset: Some(offset),
            box_type: None,
            #[cfg(feature = "std")]
            source: None,
        }
    }

    /// Creates an error with the given kind and box type context.
    pub(crate) const fn in_box(kind: ErrorKind, box_type: BoxType) -> Self {
        Self {
            kind,
            offset: None,
            box_type: Some(box_type),
            #[cfg(feature = "std")]
            source: None,
        }
    }

    /// Creates an error with kind, offset, and box type context.
    pub(crate) const fn at_in_box(kind: ErrorKind, offset: u64, box_type: BoxType) -> Self {
        Self {
            kind,
            offset: Some(offset),
            box_type: Some(box_type),
            #[cfg(feature = "std")]
            source: None,
        }
    }

    /// Adds byte offset context to this error.
    ///
    /// The offset typically represents the position in the input data
    /// where the error was detected.
    #[must_use]
    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Adds box type context to this error.
    ///
    /// This indicates which box was being processed when the error occurred.
    #[must_use]
    pub fn with_box_type(mut self, box_type: BoxType) -> Self {
        self.box_type = Some(box_type);
        self
    }

    /// Adds an underlying source error to this error.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: error::Error + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    /// Returns the error kind.
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the byte offset where the error occurred, if known.
    #[inline]
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Returns the box type being processed when the error occurred, if known.
    #[inline]
    pub fn box_type(&self) -> Option<BoxType> {
        self.box_type
    }

    /// Returns `true` if this error has an associated byte offset.
    #[inline]
    pub fn has_offset(&self) -> bool {
        self.offset.is_some()
    }

    /// Returns `true` if this error has an associated box type.
    #[inline]
    pub fn has_box_type(&self) -> bool {
        self.box_type.is_some()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if let Some(box_type) = &self.box_type {
            write!(f, " in box '{box_type}'")?;
        }

        if let Some(offset) = self.offset {
            write!(f, " at offset {offset}")?;
        }

        #[cfg(feature = "std")]
        if let Some(source) = &self.source {
            write!(f, ": {}", source)?;
        }

        Ok(())
    }
}

impl error::Error for Error {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn error::Error + 'static))
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self::new(kind)
    }
}

impl From<CursorError> for Error {
    fn from(value: CursorError) -> Self {
        let kind = match value.kind {
            CursorErrorKind::Overflow => ErrorKind::Overflow,
            CursorErrorKind::UnexpectedEof {
                expected,
                remaining,
            } => ErrorKind::NotEnoughBytes {
                expected,
                remaining,
            },
            CursorErrorKind::BufferTooSmall {
                expected,
                remaining,
            } => ErrorKind::NotEnoughBytes {
                expected,
                remaining,
            },
        };

        Self::at(kind, value.offset as u64)
    }
}

impl From<core::num::TryFromIntError> for Error {
    fn from(_err: core::num::TryFromIntError) -> Self {
        let e = Self::new(ErrorKind::Overflow);
        #[cfg(feature = "std")]
        let e = e.with_source(_err);
        e
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::new(ErrorKind::Io).with_source(value)
    }
}
