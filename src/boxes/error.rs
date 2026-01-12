//! This module defines error types and result types for BMFF box operations.

use super::boxtype::BoxType;

/// Result type for BMFF box operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Kinds of errors that can occur while processing BMFF boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The box type is invalid.
    InvalidBoxType,
    /// The box size is invalid.
    InvalidBoxSize,
    /// The box contains invalid data.
    InvalidData,
}

/// Represents an error that occurred while processing a BMFF box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    offset: Option<u64>,
    box_type: Option<BoxType>,
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
