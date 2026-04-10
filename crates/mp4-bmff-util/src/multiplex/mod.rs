//!

use core::num::NonZeroU32;

pub mod error;

mod repr;

mod context;
mod layout;

pub use context::{
    AudioSampleDescription, Context, EditSegment, FontSampleDescription, FragmentDefaults,
    HintSampleDescription, MetadataSampleDescription, SampleDescription, SubtitleSampleDescription,
    TextSampleDescription, TrackBuilder, VisualSampleDescription,
};
pub use layout::{Chunk, DataLayout};

mod compose;
mod decompose;

pub use compose::build_moov;
pub use decompose::parse_moov;

/// A type alias for the result type used in multiplexing operations, where the error type is `MuxError`.
pub type MuxError = error::Error;
type Result<T> = core::result::Result<T, MuxError>;

/// Unique identifier for a track within an MP4 file, used to associate samples and metadata with the correct track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(NonZeroU32);

impl TrackId {
    /// Creates a new `TrackId` with the given value.
    pub(super) fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }

    pub(super) fn as_u32(&self) -> u32 {
        self.0.get()
    }

    pub(super) fn as_nonzero(&self) -> NonZeroU32 {
        self.0
    }
}
