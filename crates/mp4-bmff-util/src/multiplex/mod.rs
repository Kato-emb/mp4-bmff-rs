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

mod internal;

mod error;

#[cfg(feature = "mux")]
pub mod mux;

#[cfg(feature = "demux")]
pub mod demux;

/// A type alias for the result type used in multiplexing operations, where the error type is `MuxError`.
pub type MuxError = error::Error;
type Result<T> = core::result::Result<T, MuxError>;

/// Represents a media sample in an MP4 file, containing metadata and sample data.
#[derive(Debug)]
pub struct Sample<T = Vec<u8>> {
    track_id: NonZeroU32,
    dts_ns: u64,
    pts_ns: Option<i64>,
    duration: Duration,
    is_sync: bool,
    data: T,
}

impl<T> Sample<T> {
    /// Creates a new `Sample` with the given parameters.
    ///
    /// # Panics
    /// - `track_id` must be non-zero.
    /// - The composition time offset is calculated as `pts - dts` and must fit within an `i32`. If it does not, it will default to `0`.
    pub fn new(
        track_id: u32,
        dts_ns: u64,
        pts_ns: Option<i64>,
        duration: Duration,
        is_sync: bool,
        data: T,
    ) -> Self {
        let track_id = NonZeroU32::new(track_id).expect("track_id must be non-zero");

        Self {
            track_id,
            dts_ns,
            pts_ns,
            duration,
            is_sync,
            data,
        }
    }

    /// Returns the track ID of the sample.
    pub fn track_id(&self) -> u32 {
        self.track_id.get()
    }

    /// Returns the decode time of the sample in nanoseconds.
    pub fn dts_ns(&self) -> u64 {
        self.dts_ns
    }

    /// Returns the presentation time of the sample in nanoseconds, if available.
    pub fn pts_ns(&self) -> Option<i64> {
        self.pts_ns
    }

    /// Returns the duration of the sample.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns whether the sample is a sync sample (keyframe).
    pub fn is_sync(&self) -> bool {
        self.is_sync
    }

    /// Returns a reference to the sample data.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Consumes the `Sample`, returning the sample data.
    pub fn into_data(self) -> T {
        self.data
    }

    /// Transforms the sample data using the provided function, returning a new `Sample` with the transformed data.
    pub fn map_data<U>(self, f: impl FnOnce(T) -> U) -> Sample<U> {
        Sample {
            track_id: self.track_id,
            dts_ns: self.dts_ns,
            pts_ns: self.pts_ns,
            duration: self.duration,
            is_sync: self.is_sync,
            data: f(self.data),
        }
    }
}

impl<T: AsRef<[u8]>> Sample<T> {
    /// Returns a reference to the sample data as a byte slice.
    pub fn as_ref(&self) -> Sample<&[u8]> {
        Sample {
            track_id: self.track_id,
            dts_ns: self.dts_ns,
            pts_ns: self.pts_ns,
            duration: self.duration,
            is_sync: self.is_sync,
            data: self.data.as_ref(),
        }
    }
}

/// Description of a visual (video) sample entry.
#[derive(Debug)]
pub enum VisualSampleDescription {
    /// AVC (H.264) sample entry using the `avc1` box.
    #[cfg(feature = "avc")]
    Avc1(mp4_bmff::boxes::avc::Avc1SampleEntry),
    /// AVC (H.264) sample entry using the `avc3` box (in-band parameter sets).
    #[cfg(feature = "avc")]
    Avc3(mp4_bmff::boxes::avc::Avc3SampleEntry),
    /// MPEG-4 Visual sample entry using the `mp4v` box.
    #[cfg(feature = "mp4")]
    Mp4v(mp4_bmff::boxes::mp4::Mp4vSampleEntry),
}

impl VisualSampleDescription {
    /// Returns the width of the visual sample in pixels.
    pub fn width(&self) -> u16 {
        match self {
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc1(entry) => entry.base.width,
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc3(entry) => entry.base.width,
            #[cfg(feature = "mp4")]
            VisualSampleDescription::Mp4v(entry) => entry.base.width,
        }
    }

    /// Returns the height of the visual sample in pixels.
    pub fn height(&self) -> u16 {
        match self {
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc1(entry) => entry.base.height,
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc3(entry) => entry.base.height,
            #[cfg(feature = "mp4")]
            VisualSampleDescription::Mp4v(entry) => entry.base.height,
        }
    }
}

/// Description of an audio sample entry.
#[derive(Debug)]
pub enum AudioSampleDescription {
    /// MPEG-4 Audio sample entry using the `mp4a` box.
    #[cfg(feature = "mp4")]
    Mp4a(mp4_bmff::boxes::mp4::Mp4aSampleEntry),
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
    Video(VisualSampleDescription),
    /// Audio track with an audio sample description.
    Audio(AudioSampleDescription),
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
            MediaDefinition::Video(_) => FourCC::new(*b"vide"),
            MediaDefinition::Audio(_) => FourCC::new(*b"soun"),
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
