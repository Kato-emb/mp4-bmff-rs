//! Multiplexing primitives that bridge BMFF box trees and an intermediate
//! representation centered on [`container::Movie`] / [`container::Track`].
//!
//! # Concepts
//!
//! - [`compose_moov`]: Build a [`MoovBox`](mp4_bmff::boxes::bmff::MoovBox)
//!   from a [`Movie`](container::Movie). This is the *non-fragmented* moov
//!   construction path used to finalize MP4 files.
//! - [`decompose_moov`]: Convert a parsed `moov` view into the intermediate
//!   [`Movie`](container::Movie) representation for further inspection or
//!   transformation.
//! - [`decompose_moof`]: Convert every `traf` of a `moof` view into per-track
//!   [`SampleSpec`](container::SampleSpec) /
//!   [`ChunkLayout`](container::ChunkLayout) increments. Used to remux fMP4
//!   fragments into a non-fragmented MP4.
//!
//! Higher-level orchestration (segment-by-segment accumulation, fragment
//! emission, etc.) is intentionally left out of this module and is expected to
//! live in dedicated muxer/remuxer types built on top of these primitives.

pub mod error;

/// Intermediate representation types ([`Movie`](container::Movie),
/// [`Track`](container::Track), [`SampleSpec`](container::SampleSpec),
/// [`ChunkLayout`](container::ChunkLayout), [`SampleDescription`](container::SampleDescription))
/// that act as the bridge between BMFF box trees and higher-level mux/demux/remux operations.
pub mod container;

mod compose;
mod decompose;

pub use compose::compose_moov;
pub use decompose::{decompose_moof, decompose_moov};

/// A type alias for the result type used in multiplexing operations, where the error type is `MuxError`.
pub type MuxError = error::Error;
type Result<T> = core::result::Result<T, MuxError>;
