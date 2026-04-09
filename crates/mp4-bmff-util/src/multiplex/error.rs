//! Error types for multiplexing operations.
//!
//! This module provides error handling for the muxer. Errors carry an
//! [`ErrorKind`] describing the failure category, an optional human-readable
//! message, and an optional source error for chaining.

use core::error;
use core::fmt;

use alloc::boxed::Box;
use alloc::string::String;

/// Errors that can occur during multiplexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An invalid input was provided. This can occur when the input data does not meet the expected format or constraints, such as when a sample's data length exceeds the maximum allowed size for a u32.
    InvalidInput,
    /// An arithmetic overflow occurred during calculations, such as when converting durations to ticks or calculating composition time offsets.
    Overflow,
    /// Unknown error kind, used as a fallback for non-exhaustive matching. This variant should not be constructed directly and is intended to allow for future expansion of error kinds without breaking existing code.
    __Unknown,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::InvalidInput => write!(f, "Invalid input error"),
            ErrorKind::Overflow => write!(f, "Overflow error"),
            _ => write!(f, "Unknown error"),
        }
    }
}

/// A structured error type for multiplexing operations, containing an `ErrorKind` and an optional source error for more context.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: Option<String>,
    source: Option<Box<dyn error::Error + Send + Sync + 'static>>,
}

impl Error {
    /// Creates a new `Error` with the specified `ErrorKind`.
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            message: None,
            source: None,
        }
    }

    /// Adds a custom message to the current `Error`, providing more context about the error.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds a source error to the current `Error`, providing more context.
    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: error::Error + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    /// Returns the `ErrorKind` of this error.
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if let Some(message) = &self.message {
            write!(f, ": {}", message)?;
        }

        if let Some(source) = &self.source {
            write!(f, ": {}", source)?;
        }

        Ok(())
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}
