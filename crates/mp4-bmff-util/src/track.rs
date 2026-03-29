//!

use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::*;
use mp4_bmff::types::{
    FourCC, //
    I16F16,
    LanguageCode,
    Matrix,
    U8F8,
    U16F16,
};

use super::mux::duration_to_ticks;
use crate::mux::MuxError;
use crate::sample::SampleTable;

#[derive(Debug)]
pub enum VisualSampleDescription {
    #[cfg(feature = "avc")]
    Avc1(mp4_bmff::boxes::avc::Avc1SampleEntry),
    #[cfg(feature = "avc")]
    Avc3(mp4_bmff::boxes::avc::Avc3SampleEntry),
    #[cfg(feature = "mp4")]
    Mp4v(mp4_bmff::boxes::mp4::Mp4vSampleEntry),
}

impl VisualSampleDescription {
    fn width(&self) -> u16 {
        match self {
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc1(entry) => entry.base.width,
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc3(entry) => entry.base.width,
            #[cfg(feature = "mp4")]
            VisualSampleDescription::Mp4v(entry) => entry.base.width,
        }
    }

    fn height(&self) -> u16 {
        match self {
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc1(entry) => entry.base.height,
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc3(entry) => entry.base.height,
            #[cfg(feature = "mp4")]
            VisualSampleDescription::Mp4v(entry) => entry.base.height,
        }
    }

    fn to_raw_box(&self) -> Result<RawBoxOwned, MuxError> {
        match self {
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc1(entry) => {
                Ok(RawBoxOwned::new(entry.boxtype(), entry.encode_to_vec()?))
            }
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc3(entry) => {
                Ok(RawBoxOwned::new(entry.boxtype(), entry.encode_to_vec()?))
            }
            #[cfg(feature = "mp4")]
            VisualSampleDescription::Mp4v(entry) => {
                Ok(RawBoxOwned::new(entry.boxtype(), entry.encode_to_vec()?))
            }
        }
    }
}

#[derive(Debug)]
pub enum AudioSampleDescription {
    #[cfg(feature = "mp4")]
    Mp4a(mp4_bmff::boxes::mp4::Mp4aSampleEntry),
}

impl AudioSampleDescription {
    fn to_raw_box(&self) -> Result<RawBoxOwned, MuxError> {
        match self {
            #[cfg(feature = "mp4")]
            AudioSampleDescription::Mp4a(entry) => {
                Ok(RawBoxOwned::new(entry.boxtype(), entry.encode_to_vec()?))
            }
        }
    }
}

#[derive(Debug)]
pub enum MetadataSampleDescription {}

#[derive(Debug)]
pub enum HintSampleDescription {}

#[derive(Debug)]
pub enum TextSampleDescription {}

#[derive(Debug)]
pub enum SubtitleSampleDescription {}

#[derive(Debug)]
pub enum FontSampleDescription {}

#[derive(Debug)]
pub enum MediaDefinition {
    Video(VisualSampleDescription),
    Audio(AudioSampleDescription),
    Metadata(MetadataSampleDescription),
    Hint(HintSampleDescription),
    Text(TextSampleDescription),
    Subtitle(SubtitleSampleDescription),
    Font(FontSampleDescription),
    Other(FourCC),
}

impl MediaDefinition {
    /// Returns the handler type FourCC code corresponding to this media definition.
    fn handler_type(&self) -> FourCC {
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

    fn media_header(&self) -> MediaHeaderBox {
        match self {
            MediaDefinition::Video(_) => {
                let vmhd = VmhdBox::default();
                MediaHeaderBox::Vmhd(vmhd)
            }
            MediaDefinition::Audio(_) => {
                let smhd = SmhdBox::default();
                MediaHeaderBox::Smhd(smhd)
            }
            MediaDefinition::Metadata(_) | MediaDefinition::Text(_) | MediaDefinition::Font(_) => {
                let nmhd = NmhdBox::default();
                MediaHeaderBox::Nmhd(nmhd)
            }
            MediaDefinition::Hint(_) => {
                let hmhd = HmhdBox::default();
                MediaHeaderBox::Hmhd(hmhd)
            }
            MediaDefinition::Subtitle(_) => {
                unimplemented!("SubtitleMediaHeaderBox is not yet implemented in mp4-bmff");
            }
            MediaDefinition::Other(_) => {
                // For unknown media types, we can default to a Null Media Header Box (nmhd).
                let nmhd = NmhdBox::default();
                MediaHeaderBox::Nmhd(nmhd)
            }
        }
    }

    pub(super) fn to_raw_box(&self) -> Result<RawBoxOwned, MuxError> {
        match self {
            MediaDefinition::Video(desc) => desc.to_raw_box(),
            MediaDefinition::Audio(desc) => desc.to_raw_box(),
            _ => unimplemented!(
                "to_raw_box is only implemented for video and audio sample descriptions"
            ),
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

impl EditSegment {
    fn duration(&self) -> Duration {
        match self {
            EditSegment::Empty { duration } => *duration,
            EditSegment::Media { duration, .. } => *duration,
            EditSegment::Dwell { duration, .. } => *duration,
        }
    }

    fn to_elst_entry(
        &self,
        movie_timescale: NonZeroU32,
        media_timescale: NonZeroU32,
    ) -> Result<ElstEntry, MuxError> {
        match self {
            EditSegment::Empty { duration } => {
                let segment_duration = duration_to_ticks(*duration, movie_timescale.get())
                    .ok_or(MuxError::Overflow)?;
                Ok(ElstEntry {
                    segment_duration,
                    media_time: -1,
                    media_rate: I16F16::from_f32(1.0),
                })
            }
            EditSegment::Media {
                duration,
                media_start,
                media_rate,
            } => {
                let segment_duration = duration_to_ticks(*duration, movie_timescale.get())
                    .ok_or(MuxError::Overflow)?;
                let media_time = duration_to_ticks(*media_start, media_timescale.get())
                    .and_then(|t| i64::try_from(t).ok())
                    .ok_or(MuxError::Overflow)?;
                Ok(ElstEntry {
                    segment_duration,
                    media_time,
                    media_rate: *media_rate,
                })
            }
            EditSegment::Dwell {
                duration,
                media_time,
            } => {
                let segment_duration = duration_to_ticks(*duration, movie_timescale.get())
                    .ok_or(MuxError::Overflow)?;
                let media_time = duration_to_ticks(*media_time, media_timescale.get())
                    .and_then(|t| i64::try_from(t).ok())
                    .ok_or(MuxError::Overflow)?;
                Ok(ElstEntry {
                    segment_duration,
                    media_time,
                    media_rate: I16F16::from_f32(0.0),
                })
            }
        }
    }
}

/// Represents a track in an MP4 file, containing metadata about the track.
#[derive(Debug)]
pub struct Track {
    id: NonZeroU32,
    timescale: NonZeroU32,
    language: LanguageCode,
    matrix: Matrix,
    alternate_group: i16,
    edit_list: Option<Vec<EditSegment>>,
    media: MediaDefinition,
}

impl Track {
    fn compute_tkhd_duration(
        &self,
        movie_timescale: NonZeroU32,
        media_duration: Duration,
    ) -> Result<u64, MuxError> {
        let total: Duration = if let Some(segments) = &self.edit_list {
            // If an edit list is present, the track duration is the sum of the edit segment durations.
            segments.iter().map(|seg| seg.duration()).sum()
        } else {
            // If no edit list is present, the track duration is the sum of the sample durations.
            media_duration
        };

        duration_to_ticks(total, movie_timescale.get()).ok_or(MuxError::Overflow)
    }

    fn build_tkhd(
        &self,
        movie_timescale: NonZeroU32,
        media_duration: Duration,
    ) -> Result<TkhdBox, MuxError> {
        let duration = self.compute_tkhd_duration(movie_timescale, media_duration)?;
        let mut tkhd = TkhdBox::new(self.id, duration);

        tkhd.alternate_group = self.alternate_group;
        tkhd.matrix = self.matrix;

        match &self.media {
            MediaDefinition::Video(desc) => {
                tkhd.width = U16F16::from_integer(i128::from(desc.width()));
                tkhd.height = U16F16::from_integer(i128::from(desc.height()));
            }
            MediaDefinition::Audio(_) => {
                tkhd.volume = U8F8::from_f32(1.0); // Full volume
            }
            _ => {}
        }

        Ok(tkhd)
    }

    fn compute_mdhd_duration(&self, media_duration: Duration) -> Result<u64, MuxError> {
        duration_to_ticks(media_duration, self.timescale.get()).ok_or(MuxError::Overflow)
    }

    fn build_mdhd(&self, media_duration: Duration) -> Result<MdhdBox, MuxError> {
        let duration = self.compute_mdhd_duration(media_duration)?;
        let mdhd = MdhdBox::new(self.timescale, duration, self.language);

        Ok(mdhd)
    }

    fn build_stsd(&self) -> Result<StsdBox, MuxError> {
        Ok(StsdBox {
            entries: alloc::vec![self.media.to_raw_box()?],
            ..Default::default()
        })
    }

    fn build_minf(&self, sample_table: &SampleTable) -> Result<MinfBox, MuxError> {
        let media_header = self.media.media_header();
        let stsd = self.build_stsd()?;
        let stbl = sample_table.build_stbl(stsd, self.timescale)?;
        let dinf = DinfBox::self_contained();

        Ok(MinfBox {
            media_header,
            stbl,
            dinf,
        })
    }

    fn build_mdia(
        &self,
        media_duration: Duration,
        sample_table: &SampleTable,
    ) -> Result<MdiaBox, MuxError> {
        let mdhd = self.build_mdhd(media_duration)?;
        let hdlr = HdlrBox::new(self.media.handler_type());
        let minf = self.build_minf(sample_table)?;

        Ok(MdiaBox {
            mdhd,
            hdlr,
            minf,
            elng: None,
        })
    }

    fn build_elst_entries(
        &self,
        movie_timescale: NonZeroU32,
    ) -> Result<Option<Vec<ElstEntry>>, MuxError> {
        let Some(segments) = &self.edit_list else {
            return Ok(None);
        };

        let media_timescale = self.timescale;

        segments
            .iter()
            .map(|seg| seg.to_elst_entry(movie_timescale, media_timescale))
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn build_edts(&self, movie_timescale: NonZeroU32) -> Result<Option<EdtsBox>, MuxError> {
        let Some(entries) = self.build_elst_entries(movie_timescale)? else {
            return Ok(None);
        };

        Ok(Some(EdtsBox {
            elst: Some(ElstBox::new(entries)),
        }))
    }

    pub(super) fn build_trak(
        &self,
        movie_timescale: NonZeroU32,
        sample_table: &SampleTable,
    ) -> Result<TrakBox, MuxError> {
        let media_duration = sample_table.media_duration();
        let tkhd = self.build_tkhd(movie_timescale, media_duration)?;
        let mdia = self.build_mdia(media_duration, sample_table)?;
        let edts = self.build_edts(movie_timescale)?;

        Ok(TrakBox {
            tkhd,
            tref: None,
            trgr: None,
            mdia,
            edts,
        })
    }
}

pub(super) struct SampleDefaults {
    sample_duration: Option<u32>,
    sample_size: Option<u32>,
    sample_flags: Option<SampleFlags>,
}

pub(super) struct FragmentTrack {
    track: Track,
    defaults: SampleDefaults,
}

impl FragmentTrack {
    pub(super) fn build_trak(&self, movie_timescale: NonZeroU32) -> Result<TrakBox, MuxError> {
        let tkhd = self.track.build_tkhd(movie_timescale, Duration::ZERO)?;
        let sample_table = SampleTable::empty();
        let mdia = self
            .track
            .build_mdia(sample_table.media_duration(), &sample_table)?;
        let edts = self.track.build_edts(movie_timescale)?;

        Ok(TrakBox {
            tkhd,
            tref: None,
            trgr: None,
            mdia,
            edts,
        })
    }

    pub(super) fn build_trex(&self) -> TrexBox {
        TrexBox {
            track_id: self.track.id.get(),
            default_sample_description_index: 1, // 1-based index, so 1 means the first (and only) entry in stsd
            default_sample_duration: self.defaults.sample_duration.unwrap_or(0),
            default_sample_size: self.defaults.sample_size.unwrap_or(0),
            default_sample_flags: self.defaults.sample_flags.unwrap_or_default(),
            ..Default::default()
        }
    }

    pub(super) fn build_traf(
        &self,
        sample_table: &SampleTable,
        moof_offset: u64,
    ) -> Result<TrafBox, MuxError> {
        let tfhd = TfhdBox {
            flags: TfhdFlags::DEFAULT_BASE_IS_MOOF,
            track_id: self.track.id.get(),
            ..Default::default()
        };

        let base_media_decode_time = sample_table.base_media_decode_time(self.track.timescale)?;
        let tfdt = TfdtBox::new(base_media_decode_time);

        let truns = sample_table.build_truns(
            self.track.timescale,
            moof_offset,
            self.defaults.sample_duration,
            self.defaults.sample_size,
            self.defaults.sample_flags,
        )?;

        Ok(TrafBox {
            tfhd,
            tfdt: Some(tfdt),
            truns,
            sbgps: Vec::new(),
            sgpds: Vec::new(),
        })
    }
}
