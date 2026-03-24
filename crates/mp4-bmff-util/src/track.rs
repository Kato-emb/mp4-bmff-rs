//!

use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::ElstEntry;
use mp4_bmff::types::{
    FourCC, //
    LanguageCode,
    Matrix,
};

#[derive(Debug)]
pub enum VideoCodec {
    #[cfg(feature = "avc")]
    Avc1(mp4_bmff::boxes::avc::Avc1SampleEntry),
    #[cfg(feature = "mp4")]
    Mp4v(mp4_bmff::boxes::mp4::Mp4vSampleEntry),
}

#[derive(Debug)]
pub enum MediaDescription {
    Video(VideoCodec),
    Audio,
    Metadata,
    Hint,
    Text,
    Subtitle,
    Font,
    Other(FourCC),
}

/// Represents a track in an MP4 file, containing metadata about the track.
#[derive(Debug)]
pub struct Track {
    id: NonZeroU32,
    timescale: NonZeroU32,
    duration: Duration,
    language: LanguageCode,
    matrix: Matrix,
    alternate_group: i16,
    edit_list: Option<Vec<ElstEntry>>,
    media: MediaDescription,
}
