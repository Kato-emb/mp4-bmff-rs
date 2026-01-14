//! This module defines error types and result types for BMFF box operations.

use super::boxtype::BoxType;

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
    /// Some other kind of error.
    Other {
        /// A description of the error.
        description: &'static str,
    },
}

/// Represents an error that occurred while processing a BMFF box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    offset: Option<u64>,
    box_type: Option<BoxType>,
}

impl Error {
    /// Creates a new error with the given kind.
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            offset: None,
            box_type: None,
        }
    }

    /// Sets the offset where the error occurred.
    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the box type where the error occurred.
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
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            kind,
            offset: None,
            box_type: None,
        }
    }
}

use crate::{cursor::Error as CursorError, types::FourCC};

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
