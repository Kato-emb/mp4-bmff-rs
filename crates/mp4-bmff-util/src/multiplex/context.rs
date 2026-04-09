use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::types::*;

use super::TrackId;

use super::Result;
use super::error::*;
use super::repr::*;

/// Logical metadata context for an MP4 file, holding track definitions
/// and sample timing information.
///
/// Use [`add_track`](Self::add_track) to create tracks via [`TrackBuilder`],
/// then [`add_sample`](Self::add_sample) to record sample timing, and
/// optionally [`set_edit_list`](Self::set_edit_list) to adjust playback timing.
#[derive(Debug)]
pub struct Context {
    movie: Movie,
}

impl Context {
    /// Creates a new context with the given movie timescale (ticks per second).
    ///
    /// # Errors
    ///
    /// Returns an error if `timescale` is zero.
    pub fn new(timescale: u32) -> Result<Self> {
        let Some(timescale) = NonZeroU32::new(timescale) else {
            return Err(Error::new(ErrorKind::InvalidInput)
                .with_message("Movie timescale must be a non-zero value"));
        };

        Ok(Self {
            movie: Movie {
                timescale: Timescale(timescale),
                tracks: Vec::new(),
            },
        })
    }

    /// Begins building a new track with the given timescale and initial
    /// sample description. Call [`TrackBuilder::finish`] to commit the track.
    pub fn add_track(
        &mut self,
        timescale: u32,
        description: SampleDescription,
    ) -> TrackBuilder<'_> {
        TrackBuilder {
            context: self,
            timescale,
            descriptions: alloc::vec![description],
            language: LanguageCode::UNDETERMINED,
            matrix: Matrix::identity(),
            alternate_group: 0,
        }
    }

    /// Sets the edit list for a track, replacing any previously set edit list.
    ///
    /// Edit segments use [`Duration`] values that are converted to the
    /// appropriate timescale ticks internally.
    ///
    /// # Errors
    ///
    /// Returns an error if the track ID is not found or if a timescale
    /// conversion overflows.
    pub fn set_edit_list(&mut self, track_id: TrackId, edits: &[EditSegment]) -> Result<()> {
        let movie_timescale = self.movie.timescale;
        let track = self.get_track_mut(track_id)?;
        let track_timescale = track.timescale;

        let mut edit_list = Vec::with_capacity(edits.len());
        for edit in edits {
            let converted =
                convert_edit(edit, movie_timescale, track_timescale).ok_or_else(|| {
                    Error::new(ErrorKind::Overflow)
                        .with_message("Failed to convert edit segment to timescale ticks")
                })?;
            edit_list.push(converted);
        }

        track.edit_list = Some(edit_list);
        Ok(())
    }

    /// Adds a sample to the specified track.
    ///
    /// Timing values are given in nanoseconds and converted to the track's
    /// timescale internally using a telescoping-sum approach to avoid
    /// accumulated rounding errors.
    ///
    /// # Errors
    ///
    /// Returns an error if the track ID is not found or if a timescale
    /// conversion overflows.
    pub fn add_sample(
        &mut self,
        track_id: TrackId,
        pts_ns: i64,
        dts_ns: u64,
        duration: Duration,
        is_sync: bool,
    ) -> Result<()> {
        let track = self.get_track_mut(track_id)?;
        let delta = sample_delta(dts_ns, duration, track.timescale).ok_or(
            Error::new(ErrorKind::Overflow).with_message("Sample duration calculation overflowed"),
        )?;

        let composition_time_offset =
            sample_composition_time_offset(pts_ns, dts_ns, track.timescale).ok_or(
                Error::new(ErrorKind::Overflow)
                    .with_message("Composition time offset calculation overflowed"),
            )?;

        track.samples.push(Sample {
            delta,
            is_sync,
            composition_time_offset,
        });

        Ok(())
    }

    fn next_track_id(&self) -> Option<TrackId> {
        let next_id = match self.movie.tracks.iter().map(|t| t.id).max() {
            Some(max_id) => max_id.as_u32().checked_add(1),
            None => Some(1),
        };

        next_id.and_then(TrackId::new)
    }

    fn get_track_mut(&mut self, track_id: TrackId) -> Result<&mut Track> {
        self.movie
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message("Track ID not found"))
    }
}

/// Builder for configuring a track before adding it to a [`Context`].
///
/// Created by [`Context::add_track`]. Call [`finish`](Self::finish) to
/// commit the track and obtain its [`TrackId`].
#[derive(Debug)]
pub struct TrackBuilder<'a> {
    context: &'a mut Context,
    timescale: u32,
    descriptions: Vec<SampleDescription>,
    language: LanguageCode,
    matrix: Matrix,
    alternate_group: i16,
}

impl TrackBuilder<'_> {
    /// Adds an additional sample description to the track.
    ///
    /// All descriptions must share the same handler type as the initial
    /// description provided to [`Context::add_track`].
    #[must_use]
    pub fn add_description(mut self, desc: SampleDescription) -> Self {
        self.descriptions.push(desc);
        self
    }

    /// Sets the language for the track. Defaults to undetermined.
    #[must_use]
    pub fn language(mut self, language: LanguageCode) -> Self {
        self.language = language;
        self
    }

    /// Sets the transformation matrix for the track. Defaults to identity.
    #[must_use]
    pub fn matrix(mut self, matrix: Matrix) -> Self {
        self.matrix = matrix;
        self
    }

    /// Sets the alternate group for the track. Defaults to 0 (no group).
    #[must_use]
    pub fn alternate_group(mut self, group: i16) -> Self {
        self.alternate_group = group;
        self
    }

    /// Validates the track configuration and adds the track to the context.
    ///
    /// # Errors
    ///
    /// Returns an error if the timescale is zero, sample descriptions have
    /// mismatched handler types, or the maximum track count is exceeded.
    pub fn finish(self) -> Result<TrackId> {
        let Some(track_id) = self.context.next_track_id() else {
            return Err(Error::new(ErrorKind::Overflow)
                .with_message("Exceeded maximum number of tracks (2^32 - 1)"));
        };

        let Some(timescale) = NonZeroU32::new(self.timescale) else {
            return Err(Error::new(ErrorKind::InvalidInput)
                .with_message("Track timescale must be a non-zero value"));
        };

        let handler_type = self.descriptions[0].handler_type();
        for desc in &self.descriptions[1..] {
            if desc.handler_type() != handler_type {
                return Err(Error::new(ErrorKind::InvalidInput)
                    .with_message("All sample descriptions must have the same handler type"));
            }
        }

        let track_timescale = Timescale(timescale);

        self.context.movie.tracks.push(Track {
            id: track_id,
            timescale: track_timescale,
            language: self.language,
            matrix: self.matrix,
            alternate_group: self.alternate_group,
            descriptions: self.descriptions,
            samples: Vec::new(),
            edit_list: None,
        });

        Ok(track_id)
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

fn sample_delta(dts_ns: u64, duration: Duration, timescale: Timescale) -> Option<u32> {
    let start_ns = dts_ns;
    let end_ns = u64::try_from(duration.as_nanos())
        .ok()?
        .checked_add(start_ns)?;

    timescale
        .nanos_to_ticks(end_ns)?
        .checked_sub(timescale.nanos_to_ticks(start_ns)?)
        .and_then(|delta| u32::try_from(delta).ok())
}

fn sample_composition_time_offset(pts_ns: i64, dts_ns: u64, timescale: Timescale) -> Option<i64> {
    let (abs_cto_ns, negative) = if pts_ns >= 0 {
        #[allow(clippy::cast_sign_loss)]
        let pts_uns = pts_ns as u64;
        if pts_uns >= dts_ns {
            (pts_uns - dts_ns, false)
        } else {
            (dts_ns - pts_uns, true)
        }
    } else {
        (dts_ns.checked_add(pts_ns.unsigned_abs())?, true)
    };

    let ticks = i64::try_from(timescale.nanos_to_ticks(abs_cto_ns)?).ok()?;
    Some(if negative { -ticks } else { ticks })
}

fn convert_edit(
    edit: &EditSegment,
    movie_timescale: Timescale,
    track_timescale: Timescale,
) -> Option<Edit> {
    match edit {
        EditSegment::Empty { duration } => Some(Edit {
            segment_duration: movie_timescale.duration_to_ticks(*duration)?,
            media_time: -1,
            media_rate: I16F16::from_f32(1.0),
        }),
        EditSegment::Media {
            duration,
            media_start,
            media_rate,
        } => Some(Edit {
            segment_duration: movie_timescale.duration_to_ticks(*duration)?,
            media_time: i64::try_from(track_timescale.duration_to_ticks(*media_start)?).ok()?,
            media_rate: I16F16::from_f32(*media_rate),
        }),
        EditSegment::Dwell {
            duration,
            media_time,
        } => Some(Edit {
            segment_duration: movie_timescale.duration_to_ticks(*duration)?,
            media_time: i64::try_from(track_timescale.duration_to_ticks(*media_time)?).ok()?,
            media_rate: I16F16::from_f32(0.0),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_desc() -> SampleDescription {
        SampleDescription::Other(FourCC::new(*b"vide"))
    }

    fn audio_desc() -> SampleDescription {
        SampleDescription::Other(FourCC::new(*b"soun"))
    }

    // --- Context::new ---

    #[test]
    fn new_with_valid_timescale() {
        assert!(Context::new(1000).is_ok());
    }

    #[test]
    fn new_with_zero_timescale_returns_error() {
        assert!(Context::new(0).is_err());
    }

    // --- TrackBuilder ---

    #[test]
    fn add_track_returns_incrementing_ids() {
        let mut ctx = Context::new(1000).unwrap();
        let id1 = ctx.add_track(90000, video_desc()).finish().unwrap();
        let id2 = ctx.add_track(48000, audio_desc()).finish().unwrap();

        assert_eq!(id1.as_u32(), 1);
        assert_eq!(id2.as_u32(), 2);
    }

    #[test]
    fn add_track_with_zero_timescale_returns_error() {
        let mut ctx = Context::new(1000).unwrap();
        let result = ctx.add_track(0, video_desc()).finish();
        assert!(result.is_err());
    }

    #[test]
    fn add_track_with_language() {
        let mut ctx = Context::new(1000).unwrap();
        let lang = LanguageCode::new(*b"eng");
        let id = ctx
            .add_track(90000, video_desc())
            .language(lang)
            .finish()
            .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        assert_eq!(track.language, lang);
    }

    #[test]
    fn add_track_with_alternate_group() {
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx
            .add_track(48000, audio_desc())
            .alternate_group(1)
            .finish()
            .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        assert_eq!(track.alternate_group, 1);
    }

    #[test]
    fn add_track_with_multiple_descriptions() {
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx
            .add_track(90000, video_desc())
            .add_description(video_desc())
            .finish()
            .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        assert_eq!(track.descriptions.len(), 2);
    }

    #[test]
    fn add_track_mismatched_descriptions_returns_error() {
        let mut ctx = Context::new(1000).unwrap();
        let result = ctx
            .add_track(90000, video_desc())
            .add_description(audio_desc())
            .finish();
        assert!(result.is_err());
    }

    // --- add_sample ---

    #[test]
    fn add_sample_basic() {
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx.add_track(90000, video_desc()).finish().unwrap();

        ctx.add_sample(id, 0, 0, Duration::from_millis(33), true)
            .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        assert_eq!(track.samples.len(), 1);
        assert!(track.samples[0].is_sync);
    }

    #[test]
    fn add_sample_invalid_track_returns_error() {
        let mut ctx = Context::new(1000).unwrap();
        let bad_id = TrackId::new(99).unwrap();
        let result = ctx.add_sample(bad_id, 0, 0, Duration::from_millis(33), true);
        assert!(result.is_err());
    }

    #[test]
    fn add_sample_telescoping_sum() {
        // Verify that deltas sum correctly via telescoping:
        // sum of deltas == ticks(end) - ticks(start)
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx.add_track(90000, video_desc()).finish().unwrap();

        let frame_ns: u64 = 1_000_000_000 / 30; // 33_333_333 ns
        let n = 3u64;
        for i in 0..n {
            let dts = frame_ns * i;
            #[allow(clippy::cast_possible_wrap)]
            ctx.add_sample(id, dts as i64, dts, Duration::from_nanos(frame_ns), true)
                .unwrap();
        }

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        let total_delta: u64 = track.samples.iter().map(|s| u64::from(s.delta)).sum();

        // Telescoping guarantee: sum == ticks(n * frame_ns) - ticks(0)
        let ts = Timescale(NonZeroU32::new(90000).unwrap());
        let expected = ts.nanos_to_ticks(frame_ns * n).unwrap();
        assert_eq!(total_delta, expected);
    }

    #[test]
    fn add_sample_with_positive_cto() {
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx.add_track(90000, video_desc()).finish().unwrap();

        // PTS ahead of DTS by 66ms
        let dts_ns: u64 = 0;
        let pts_ns: i64 = 66_000_000;
        ctx.add_sample(id, pts_ns, dts_ns, Duration::from_millis(33), false)
            .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        assert!(track.samples[0].composition_time_offset > 0);
    }

    #[test]
    fn add_sample_with_negative_cto() {
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx.add_track(90000, video_desc()).finish().unwrap();

        // PTS behind DTS
        let dts_ns: u64 = 66_000_000;
        let pts_ns: i64 = 0;
        ctx.add_sample(id, pts_ns, dts_ns, Duration::from_millis(33), false)
            .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        assert!(track.samples[0].composition_time_offset < 0);
    }

    // --- set_edit_list ---

    #[test]
    fn set_edit_list_empty_edit() {
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx.add_track(90000, video_desc()).finish().unwrap();

        ctx.set_edit_list(
            id,
            &[EditSegment::Empty {
                duration: Duration::from_secs(1),
            }],
        )
        .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        let edits = track.edit_list.as_ref().unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].media_time, -1);
        assert_eq!(edits[0].segment_duration, 1000); // movie timescale = 1000
    }

    #[test]
    fn set_edit_list_replaces_previous() {
        let mut ctx = Context::new(1000).unwrap();
        let id = ctx.add_track(90000, video_desc()).finish().unwrap();

        ctx.set_edit_list(
            id,
            &[EditSegment::Empty {
                duration: Duration::from_secs(1),
            }],
        )
        .unwrap();

        ctx.set_edit_list(
            id,
            &[
                EditSegment::Empty {
                    duration: Duration::from_secs(2),
                },
                EditSegment::Media {
                    duration: Duration::from_secs(5),
                    media_start: Duration::ZERO,
                    media_rate: 1.0,
                },
            ],
        )
        .unwrap();

        let track = ctx.movie.tracks.iter().find(|t| t.id == id).unwrap();
        assert_eq!(track.edit_list.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn set_edit_list_invalid_track_returns_error() {
        let mut ctx = Context::new(1000).unwrap();
        let bad_id = TrackId::new(99).unwrap();
        let result = ctx.set_edit_list(
            bad_id,
            &[EditSegment::Empty {
                duration: Duration::from_secs(1),
            }],
        );
        assert!(result.is_err());
    }
}
