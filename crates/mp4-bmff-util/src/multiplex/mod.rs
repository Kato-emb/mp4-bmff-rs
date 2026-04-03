//! Multiplexing and demultiplexing utilities for MP4 files.
//!
//! This module provides high-level APIs for constructing MP4 box structures
//! from raw media samples. It supports both non-fragmented (`moov` + `mdat`)
//! and fragmented (`moov` + `moof`/`mdat` pairs) output.
//!
//! # Modules
//!
//! - [`mux`]: Muxer for building MP4 box metadata from media samples
//! - [`error`]: Error types for multiplexing operations

use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::types::FourCC;

mod error;

#[cfg(feature = "mux")]
pub mod mux;

#[cfg(feature = "demux")]
pub mod demux;

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

    /// Returns the underlying `u32` value of the `TrackId`.
    pub fn get(&self) -> u32 {
        self.0.get()
    }

    pub(super) fn into_inner(self) -> NonZeroU32 {
        self.0
    }
}

/// Represents a media sample in an MP4 file, containing metadata and sample data.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    dts_ns: u64,
    pts_ns: Option<i64>,
    duration: Duration,
    is_sync: bool,
    size: u32,
}

impl Sample {
    /// Creates a new `Sample` with the given parameters.
    pub fn new(
        dts_ns: u64,
        pts_ns: Option<i64>,
        duration: Duration,
        is_sync: bool,
        size: u32,
    ) -> Self {
        Self {
            dts_ns,
            pts_ns,
            duration,
            is_sync,
            size,
        }
    }
}

/// Description of a visual (video) sample entry.
#[derive(Debug)]
pub enum VisualSampleDescription {
    /// AVC (H.264) sample entry using the `avc1` box.
    #[cfg(feature = "avc")]
    Avc1(mp4_bmff::formats::mpeg4::codecs::avc::AVCDecoderConfigurationRecord),
    /// AVC (H.264) sample entry using the `avc3` box (in-band parameter sets).
    #[cfg(feature = "avc")]
    Avc3(mp4_bmff::formats::mpeg4::codecs::avc::AVCDecoderConfigurationRecord),
    /// MPEG-4 Visual sample entry using the `mp4v` box.
    #[cfg(feature = "mp4")]
    Mp4v(Vec<u8>),
}

/// Description of an audio sample entry.
#[derive(Debug)]
pub enum AudioSampleDescription {
    /// MPEG-4 Audio sample entry using the `mp4a` box.
    #[cfg(feature = "mp4")]
    Mp4a(Vec<u8>),
}

/// Description of a metadata sample entry (placeholder).
#[derive(Debug)]
pub enum MetadataSampleDescription {}

/// Description of a hint sample entry (placeholder).
#[derive(Debug)]
pub enum HintSampleDescription {}

/// Description of a text sample entry (placeholder).
#[derive(Debug)]
pub enum TextSampleDescription {}

/// Description of a subtitle sample entry (placeholder).
#[derive(Debug)]
pub enum SubtitleSampleDescription {}

/// Description of a font sample entry (placeholder).
#[derive(Debug)]
pub enum FontSampleDescription {}

/// Defines the media type and codec-specific description for a track.
#[derive(Debug)]
pub enum MediaDefinition {
    /// Video track with a visual sample description.
    Video {
        /// Width of the video frames in pixels.
        width: u16,
        /// Height of the video frames in pixels.
        height: u16,
        /// Codec-specific description of the visual samples.
        codec: VisualSampleDescription,
    },
    /// Audio track with an audio sample description.
    Audio {
        /// Number of audio channels (e.g., 2 for stereo).
        channel_count: u16,
        /// Sample rate in Hz (e.g., 44100 for CD-quality audio).
        sample_rate: u16,
        /// Codec-specific description of the audio samples.
        codec: AudioSampleDescription,
    },
    /// Metadata track.
    Metadata(MetadataSampleDescription),
    /// Hint track (streaming hints).
    Hint(HintSampleDescription),
    /// Text track.
    Text(TextSampleDescription),
    /// Subtitle track.
    Subtitle(SubtitleSampleDescription),
    /// Font track.
    Font(FontSampleDescription),
    /// Other media type identified by a FourCC code.
    Other(FourCC),
}

impl MediaDefinition {
    /// Returns the handler type FourCC code corresponding to this media definition.
    pub fn handler_type(&self) -> FourCC {
        match self {
            MediaDefinition::Video { .. } => FourCC::new(*b"vide"),
            MediaDefinition::Audio { .. } => FourCC::new(*b"soun"),
            MediaDefinition::Metadata(_) => FourCC::new(*b"meta"),
            MediaDefinition::Hint(_) => FourCC::new(*b"hint"),
            MediaDefinition::Text(_) => FourCC::new(*b"text"),
            MediaDefinition::Subtitle(_) => FourCC::new(*b"subt"),
            MediaDefinition::Font(_) => FourCC::new(*b"fdsm"),
            MediaDefinition::Other(fourcc) => *fourcc,
        }
    }
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
        media_rate: f32,
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

impl EditSegment {
    /// Returns the duration of this edit segment.
    pub fn duration(&self) -> Duration {
        match self {
            EditSegment::Empty { duration } => *duration,
            EditSegment::Media { duration, .. } => *duration,
            EditSegment::Dwell { duration, .. } => *duration,
        }
    }
}
