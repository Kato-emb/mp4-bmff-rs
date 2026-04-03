//! Error types for multiplexing operations.
//!
//! This module provides error handling for the muxer. Errors carry an
//! [`ErrorKind`] describing the failure category, an optional human-readable
//! message, and an optional source error for chaining.

use core::error;
use core::fmt;

use alloc::boxed::Box;
use alloc::string::String;

use super::TrackId;

/// Errors that can occur during multiplexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// An arithmetic overflow occurred during calculations, such as when converting durations to ticks or calculating composition time offsets.
    Overflow,
    /// An invalid input was provided. This can occur when the input data does not meet the expected format or constraints, such as when a sample's data length exceeds the maximum allowed size for a u32.
    InvalidInput,
    /// A specified track was not found in the movie when attempting to add a sample or perform an operation on it.
    TrackNotFound(TrackId),
    /// An unsupported media type or configuration was encountered that cannot be processed by the muxer.
    Unsupported,
    /// An error occurred while encoding a box.
    BoxEncode,
    /// An error occurred while decoding a box.
    BoxDecode,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Overflow => write!(f, "Overflow error"),
            ErrorKind::InvalidInput => write!(f, "Invalid input error"),
            ErrorKind::TrackNotFound(track_id) => {
                write!(f, "Track with ID {} not found", track_id.get())
            }
            ErrorKind::Unsupported => write!(f, "Unsupported media type or configuration"),
            ErrorKind::BoxEncode => write!(f, "Box encoding error"),
            ErrorKind::BoxDecode => write!(f, "Box decoding error"),
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

    /// Creates a new `Error` with the specified `ErrorKind` and a source error for more context.
    pub fn box_decode(source: mp4_bmff::Error) -> Self {
        Error::new(ErrorKind::BoxDecode).with_source(source)
    }

    /// Creates a new `Error` with the specified `ErrorKind` and a source error for more context.
    pub fn box_encode(source: mp4_bmff::Error) -> Self {
        Error::new(ErrorKind::BoxEncode).with_source(source)
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

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error::new(kind)
    }
}

impl From<core::num::TryFromIntError> for Error {
    fn from(value: core::num::TryFromIntError) -> Self {
        Error {
            kind: ErrorKind::Overflow,
            message: None,
            source: Some(Box::new(value)),
        }
    }
}
