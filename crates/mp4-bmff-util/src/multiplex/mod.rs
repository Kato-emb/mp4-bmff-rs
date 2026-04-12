pub mod error;

pub mod container;
pub mod layout;

mod compose;
mod decompose;

pub use decompose::parse_movie;

mod demux;
mod mux;

/// A type alias for the result type used in multiplexing operations, where the error type is `MuxError`.
pub type MuxError = error::Error;
type Result<T> = core::result::Result<T, MuxError>;
