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

use mp4_bmff::types::{FourCC, LanguageCode, Matrix};

mod error;
mod repr;

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
    pts_ns: i64,
    duration: Duration,
    is_sync: bool,
    size: u32,
    description_index: NonZeroU32,
}

impl Sample {
    /// Creates a new `Sample` with the given parameters.
    pub fn new(
        dts_ns: u64,
        pts_ns: i64,
        duration: Duration,
        is_sync: bool,
        size: u32,
    ) -> Result<Self> {
        if duration.is_zero() {
            return Err(MuxError::new(error::ErrorKind::InvalidInput)
                .with_message("Sample duration must be greater than zero"));
        }

        Ok(Self {
            dts_ns,
            pts_ns,
            duration,
            is_sync,
            size,
            description_index: NonZeroU32::new(1).unwrap(), // Default to 1-based index for the first sample description
        })
    }

    /// Sets the sample description index for this sample, which indicates which sample description entry in the track's sample description box applies to this sample. The index is 1-based, meaning that a value of 1 refers to the first entry in the sample description box.
    pub fn with_description_index(mut self, index: u32) -> Result<Self> {
        self.description_index = NonZeroU32::new(index).ok_or(
            MuxError::new(error::ErrorKind::InvalidInput)
                .with_message("Description index must be non-zero"),
        )?;

        Ok(self)
    }

    /// Returns the decoding timestamp (DTS) of the sample in nanoseconds.
    pub fn dts_ns(&self) -> u64 {
        self.dts_ns
    }

    /// Returns the presentation timestamp (PTS) of the sample in nanoseconds.
    pub fn pts_ns(&self) -> i64 {
        self.pts_ns
    }

    /// Returns the duration of the sample.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns whether this sample is a sync sample (keyframe).
    pub fn is_sync(&self) -> bool {
        self.is_sync
    }

    /// Returns the size of the sample in bytes.
    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn description_index(&self) -> u32 {
        self.description_index.get()
    }
}

/// A chunk of samples that are stored together in the MP4 file, typically corresponding to a contiguous range of samples in a track.
#[derive(Debug, Clone)]
pub struct Chunk {
    data_offset: u64,
    samples: Vec<Sample>,
}

impl Chunk {
    /// Returns the byte offset in the file where the sample data for this chunk starts.
    pub fn data_offset(&self) -> u64 {
        self.data_offset
    }

    /// Returns the number of samples in this chunk.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Returns a reference to the first sample in the chunk.
    pub fn first_sample(&self) -> &Sample {
        self.samples.first().unwrap()
    }

    /// Returns a reference to the last sample in the chunk. If the chunk contains only one sample, this will return the same sample as `first_sample()`.
    pub fn last_sample(&self) -> &Sample {
        self.samples.last().unwrap()
    }

    /// Returns a reference to all samples in the chunk.
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }

    /// Attempts to add a new sample to the end of the chunk. The new sample's DTS must be greater than or equal to the last sample's DTS to maintain non-decreasing order.
    pub fn try_push_sample(&mut self, sample: Sample) -> Result<()> {
        if sample.dts_ns < self.last_sample().dts_ns {
            return Err(MuxError::new(error::ErrorKind::InvalidInput).with_message(format!(
                "Sample DTS {} is less than the last sample's DTS {} in the chunk; samples within a chunk must be in non-decreasing DTS order",
                sample.dts_ns, self.last_sample().dts_ns
            )));
        }

        self.samples.push(sample);
        Ok(())
    }
}

/// Represents a track in an MP4 file, containing metadata about the track and its associated media samples.
#[derive(Debug, Clone)]
pub struct Track {
    id: TrackId,
    timescale: NonZeroU32,
    language: LanguageCode,
    matrix: Matrix,
    alternate_group: i16,
    descriptions: Vec<SampleDescription>,
    samples: Vec<Sample>,
    edit_list: Option<Vec<EditSegment>>,
}

impl Track {
    pub(super) fn new(
        id: TrackId,
        timescale: NonZeroU32,
        descriptions: Vec<SampleDescription>,
    ) -> Self {
        Self {
            id,
            timescale,
            language: LanguageCode::default(),
            matrix: Matrix::default(),
            alternate_group: 0,
            descriptions,
            samples: Vec::new(),
            edit_list: None,
        }
    }

    /// Returns the unique identifier for this track.
    pub fn id(&self) -> TrackId {
        self.id
    }

    /// Returns the timescale for this track, which defines the time units used for sample timestamps and durations.
    pub fn timescale(&self) -> u32 {
        self.timescale.get()
    }

    /// Returns the language code for this track, indicating the language of the media content (e.g., "eng" for English).
    pub fn language(&self) -> LanguageCode {
        self.language
    }

    /// Returns the transformation matrix for this track, which can be used to specify spatial transformations for video tracks (e.g., rotation, scaling).
    pub fn matrix(&self) -> Matrix {
        self.matrix
    }

    /// Returns the alternate group for this track, which can be used to group tracks that are alternatives to each other (e.g., multiple audio tracks in different languages). A value of 0 indicates that the track does not belong to any alternate group.
    pub fn alternate_group(&self) -> i16 {
        self.alternate_group
    }

    /// Returns the media definition for this track, which describes the type of media (e.g., video, audio) and codec-specific information needed to decode the samples.
    pub fn description(&self, index: usize) -> Option<&SampleDescription> {
        let index = index.checked_sub(1)?; // Convert from 1-based to 0-based index
        self.descriptions.get(index)
    }

    /// Returns the edit list for this track, if present. The edit list defines how the media samples should be presented in the timeline, allowing for operations like inserting gaps, repeating frames, or changing playback speed.
    pub fn edit_list(&self) -> Option<&[EditSegment]> {
        self.edit_list.as_deref()
    }

    pub(super) fn edit_duration(&self) -> Option<Duration> {
        self.edit_list()
            .map(|segments| segments.iter().map(|s| s.duration()).sum())
    }
}

/// Description of a visual (video) sample entry.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum AudioSampleDescription {
    /// MPEG-4 Audio sample entry using the `mp4a` box.
    #[cfg(feature = "mp4")]
    Mp4a(Vec<u8>),
}

/// Description of a metadata sample entry (placeholder).
#[derive(Debug, Clone)]
pub enum MetadataSampleDescription {}

/// Description of a hint sample entry (placeholder).
#[derive(Debug, Clone)]
pub enum HintSampleDescription {}

/// Description of a text sample entry (placeholder).
#[derive(Debug, Clone)]
pub enum TextSampleDescription {}

/// Description of a subtitle sample entry (placeholder).
#[derive(Debug, Clone)]
pub enum SubtitleSampleDescription {}

/// Description of a font sample entry (placeholder).
#[derive(Debug, Clone)]
pub enum FontSampleDescription {}

/// Defines the media type and codec-specific description for a track.
#[derive(Debug, Clone)]
pub enum SampleDescription {
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

impl SampleDescription {
    /// Returns the handler type FourCC code corresponding to this media definition.
    pub fn handler_type(&self) -> FourCC {
        match self {
            SampleDescription::Video { .. } => FourCC::new(*b"vide"),
            SampleDescription::Audio { .. } => FourCC::new(*b"soun"),
            SampleDescription::Metadata(_) => FourCC::new(*b"meta"),
            SampleDescription::Hint(_) => FourCC::new(*b"hint"),
            SampleDescription::Text(_) => FourCC::new(*b"text"),
            SampleDescription::Subtitle(_) => FourCC::new(*b"subt"),
            SampleDescription::Font(_) => FourCC::new(*b"fdsm"),
            SampleDescription::Other(fourcc) => *fourcc,
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
