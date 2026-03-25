//!

use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::types::{
    FourCC, //
    I16F16,
    LanguageCode,
    Matrix,
};

#[derive(Debug)]
pub enum VisualSampleDescription {
    #[cfg(feature = "avc")]
    Avc1(mp4_bmff::boxes::avc::Avc1SampleEntry),
    #[cfg(feature = "avc")]
    Avc3(mp4_bmff::boxes::avc::Avc3SampleEntry),
    #[cfg(feature = "mp4")]
    Mp4v(mp4_bmff::boxes::mp4::Mp4vSampleEntry),
}

#[derive(Debug)]
pub enum AudioSampleDescription {
    #[cfg(feature = "mp4")]
    Mp4a(mp4_bmff::boxes::mp4::Mp4aSampleEntry),
}

#[derive(Debug)]
pub enum MediaDefinition {
    Video(VisualSampleDescription),
    Audio(AudioSampleDescription),
    Metadata,
    Hint,
    Text,
    Subtitle,
    Font,
    Other(FourCC),
}

/// An edit segment in the track timeline, representing a timescale-independent
/// interpretation of an Edit List Box (`elst`) entry.
#[derive(Debug, Clone)]
pub enum EditSegment {
    /// Empty edit — no media is presented for the given duration.
    ///
    /// Corresponds to an `elst` entry with `media_time == -1`.
    /// Typically used to insert a delay before the media starts playing.
    Empty {
        /// Duration of the empty gap in the presentation timeline.
        duration: Duration,
    },
    /// Normal playback of media from a given starting point.
    ///
    /// Corresponds to an `elst` entry with `media_time >= 0` and `media_rate != 0`.
    Media {
        /// Duration of this segment in the presentation timeline.
        duration: Duration,
        /// Starting point in the media timeline.
        media_start: Duration,
        /// Playback rate (1.0 = normal speed).
        media_rate: I16F16,
    },
    /// Dwell (freeze frame) — a single frame is held for the given duration.
    ///
    /// Corresponds to an `elst` entry with `media_rate == 0`.
    /// The media timeline does not advance.
    Dwell {
        /// Duration to hold the frame in the presentation timeline.
        duration: Duration,
        /// The media time of the frame to display.
        media_time: Duration,
    },
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
    edit_list: Option<Vec<EditSegment>>,
    media: MediaDefinition,
}
