use mp4_bmff::types::FourCC;

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
