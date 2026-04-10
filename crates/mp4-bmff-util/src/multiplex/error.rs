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
    /// The input data is in an invalid format. This can occur when the muxer encounters data that cannot be parsed or does not conform to the expected structure, such as when a sample description is missing required fields or contains unsupported values.
    InvalidFormat,
    /// An arithmetic overflow occurred during calculations, such as when converting durations to ticks or calculating composition time offsets.
    Overflow,
    /// An unsupported operation was attempted. This can occur when the muxer encounters a feature or format that it does not support.
    Unsupported,
    /// An error occurred during encoding. This can occur when the muxer fails to encode a sample or box.
    Encode,
    /// An error occurred during decoding. This can occur when the muxer fails to decode a sample or box.
    Decode,
    /// An error occurred in the BMFF (ISO Base Media File Format) processing. This can occur when the muxer encounters an issue specific to BMFF structures.
    Bmff,
    /// Unknown error kind, used as a fallback for non-exhaustive matching. This variant should not be constructed directly and is intended to allow for future expansion of error kinds without breaking existing code.
    __Unknown,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::InvalidInput => write!(f, "Invalid input error"),
            ErrorKind::InvalidFormat => write!(f, "Invalid format error"),
            ErrorKind::Overflow => write!(f, "Overflow error"),
            ErrorKind::Unsupported => write!(f, "Unsupported operation error"),
            ErrorKind::Encode => write!(f, "Encoding error"),
            ErrorKind::Decode => write!(f, "Decoding error"),
            ErrorKind::Bmff => write!(f, "BMFF processing error"),
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

impl From<mp4_bmff::Error> for Error {
    fn from(value: mp4_bmff::Error) -> Self {
        Self::new(ErrorKind::Bmff).with_source(value)
    }
}
