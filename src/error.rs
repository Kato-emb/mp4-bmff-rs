//! This module defines error types and result types for BMFF box operations.

use core::error;
use core::fmt;

use crate::cursor::Error as CursorError;
use crate::types::FourCC;

use crate::header::boxsize::BoxSizeError;
use crate::header::boxtype::BoxTypeError;
use crate::header::{BoxSize, BoxType};

/// Result type for BMFF box operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Kinds of errors that can occur while processing BMFF boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// An integer overflow occurred.
    Overflow,
    /// There were not enough bytes to complete the operation.
    NotEnoughBytes {
        /// Number of bytes expected.
        expected: usize,
        /// Number of bytes remaining.
        remaining: usize,
    },
    /// Mismatched box size.
    MismatchedBoxSize {
        /// Expected size.
        expected: u64,
        /// Found size.
        found: u64,
    },
    /// Mismatched box type.
    MissmatchedBoxType {
        /// Expected box type.
        expected: BoxType,
        /// Found box type.
        found: BoxType,
    },
    /// Invalid box size.
    InvalidBoxSize {
        /// A description of the invalid size.
        reason: &'static str,
        /// The invalid size encountered.
        got: u64,
    },
    /// Invalid box type.
    InvalidBoxType {
        /// A description of the invalid box type.
        reason: &'static str,
        /// The invalid box type encountered.
        got: FourCC,
    },
    /// Invalid box version.
    InvalidBoxVersion {
        /// A description of the invalid version.
        reason: &'static str,
        /// The invalid version encountered.
        got: u8,
    },
    /// Invalid box flags.
    InvalidBoxFlags {
        /// A description of the invalid flags.
        reason: &'static str,
        /// The invalid flags encountered.
        got: u32,
    },
    /// Invalid box field.
    InvalidBoxField {
        /// The name of the invalid field.
        field: &'static str,
        /// A description of the reason why the field is invalid.
        reason: &'static str,
    },
    /// A required box is missing.
    BoxMissing {
        /// The type of the required box.
        required: BoxType,
    },
    /// Some other kind of error.
    Other {
        /// A description of the error.
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
            ErrorKind::MismatchedBoxSize { expected, found } => {
                write!(f, "mismatched box size: expected {expected}, found {found}")
            }
            ErrorKind::MissmatchedBoxType { expected, found } => {
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
            ErrorKind::Other { description } => write!(f, "error: {description}"),
        }
    }
}

/// Represents an error that occurred while processing a BMFF box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    offset: Option<u64>,
    box_type: Option<BoxType>,
}

impl Error {
    pub(crate) const fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            offset: None,
            box_type: None,
        }
    }

    pub(crate) const fn at(kind: ErrorKind, offset: u64) -> Self {
        Self {
            kind,
            offset: Some(offset),
            box_type: None,
        }
    }

    pub(crate) const fn in_box(kind: ErrorKind, box_type: BoxType) -> Self {
        Self {
            kind,
            offset: None,
            box_type: Some(box_type),
        }
    }

    pub(crate) const fn at_in_box(kind: ErrorKind, offset: u64, box_type: BoxType) -> Self {
        Self {
            kind,
            offset: Some(offset),
            box_type: Some(box_type),
        }
    }

    /// Sets the offset where the error occurred.
    #[must_use]
    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the box type where the error occurred.
    #[must_use]
    pub fn with_box_type(mut self, box_type: BoxType) -> Self {
        self.box_type = Some(box_type);
        self
    }

    /// Returns the kind of error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the offset where the error occurred, if available.
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Returns the box type where the error occurred, if available.
    pub fn box_type(&self) -> Option<BoxType> {
        self.box_type
    }

    /// Returns `true` if the error has an associated offset.
    pub fn has_offset(&self) -> bool {
        self.offset.is_some()
    }

    /// Returns `true` if the error has an associated box type.
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

        Ok(())
    }
}

impl error::Error for Error {}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self::new(kind)
    }
}

impl From<CursorError> for ErrorKind {
    fn from(value: CursorError) -> Self {
        match value {
            CursorError::Overflow => ErrorKind::Overflow,
            CursorError::UnexpectedEof {
                expected,
                remaining,
            } => ErrorKind::NotEnoughBytes {
                expected,
                remaining,
            },
            CursorError::BufferTooSmall {
                expected,
                remaining,
            } => ErrorKind::NotEnoughBytes {
                expected,
                remaining,
            },
        }
    }
}

impl From<CursorError> for Error {
    fn from(value: CursorError) -> Self {
        Self::new(ErrorKind::from(value))
    }
}

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

impl From<BoxSizeError> for Error {
    fn from(value: BoxSizeError) -> Self {
        Self::new(ErrorKind::from(value))
    }
}

impl From<BoxTypeError> for ErrorKind {
    fn from(value: BoxTypeError) -> Self {
        use crate::header::boxtype::UUID;

        match value {
            BoxTypeError::UuidFourCCNotAllowed => Self::InvalidBoxType {
                reason: "Cannot create UUID BoxType from FourCC code 'uuid'",
                got: UUID,
            },
        }
    }
}

impl From<BoxTypeError> for Error {
    fn from(value: BoxTypeError) -> Self {
        Self::new(ErrorKind::from(value))
    }
}
