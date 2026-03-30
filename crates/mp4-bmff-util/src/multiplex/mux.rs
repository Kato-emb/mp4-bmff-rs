//! MP4 muxer implementation.
//!
//! This module provides [`Muxer`] for non-fragmented MP4 and [`FragmentedMuxer`]
//! for fragmented MP4 (fMP4) output. Both are constructed through a shared
//! [`Builder`] that configures movie-level timescale and per-track settings.
//!
//! The muxers accept [`Sample`](super::Sample) values and produce the
//! corresponding box structures (`MoovBox`, `MoofBox`) ready for encoding.

use core::num::NonZeroU32;

use alloc::vec::Vec;

use mp4_bmff::BoxCodec;
use mp4_bmff::BoxEncode;
use mp4_bmff::BoxHeader;
use mp4_bmff::BoxType;
use mp4_bmff::RawBoxOwned;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::*;

use crate::multiplex::Sample;

use super::Result;
use super::error::*;
use super::internal::*;

use super::AudioSampleDescription;
use super::EditSegment;
use super::MediaDefinition;
use super::VisualSampleDescription;

/// A builder for configuring and constructing a [`Muxer`] or [`FragmentedMuxer`].
#[derive(Debug)]
#[must_use = "Builder is a builder for configuring the Muxer; call build() to finalize the Muxer"]
pub struct Builder {
    movie: Movie,
}

impl Builder {
    /// Creates a new `Builder` with the specified timescale for the movie.
    pub fn new(timescale: u32) -> Result<Self> {
        let timescale = NonZeroU32::new(timescale).ok_or(
            Error::new(ErrorKind::InvalidInput).with_message("Timescale must be non-zero"),
        )?;

        Ok(Self {
            movie: Movie::new(timescale),
        })
    }

    /// Adds a new track to the movie with the given timescale and media definition, returning the assigned track ID.
    pub fn add_track(
        &mut self,
        timescale: u32,
        media: MediaDefinition,
    ) -> Result<TrackBuilder<'_>> {
        let track_id = NonZeroU32::new(self.movie.next_track_id().ok_or(ErrorKind::Overflow)?)
            .expect("next_track_id should never return 0 since it starts at 1 and increments");
        let timescale = NonZeroU32::new(timescale).ok_or(
            Error::new(ErrorKind::InvalidInput).with_message("Timescale must be non-zero"),
        )?;
        let track = Track::new(track_id, timescale, media);

        Ok(TrackBuilder {
            builder: self,
            track,
        })
    }

    /// Builds the `Muxer` from the current state of the `Builder`.
    pub fn build(self) -> Result<Muxer> {
        Ok(Muxer { movie: self.movie })
    }

    /// Builds a `FragmentedMuxer` along with the initial `MoovBox` for fragmented MP4 output.
    pub fn build_fragmented(self) -> Result<(MoovBox, FragmentedMuxer)> {
        debug_assert!(
            self.movie
                .tracks
                .iter()
                .all(|t| t.sample_table.chunks.is_empty()),
            "Fragmented movies should not have samples in the MovieBox; samples go in separate MovieFragment boxes (moof)"
        );

        let mut moov = build_moov(&self.movie)?;
        moov.mvex = Some(build_mvex(&self.movie)?);

        Ok((
            moov,
            FragmentedMuxer {
                movie: self.movie,
                sequence_number: 0,
                mdat_payload_size: 0,
            },
        ))
    }
}

/// A builder for configuring and adding a track to the movie before finalizing the `Muxer`.
#[derive(Debug)]
#[must_use = "TrackBuilder is a builder for configuring a track; call build() to finalize the track and add it to the movie"]
pub struct TrackBuilder<'a> {
    builder: &'a mut Builder,
    track: Track,
}

impl TrackBuilder<'_> {
    /// Sets the language code for the track and returns the updated `TrackBuilder`.
    pub fn language(mut self, language: LanguageCode) -> Self {
        self.track.language = language;
        self
    }

    /// Sets the matrix for the track and returns the updated `TrackBuilder`.
    pub fn matrix(mut self, matrix: Matrix) -> Self {
        self.track.matrix = matrix;
        self
    }

    /// Sets the alternate group for the track and returns the updated `TrackBuilder`.
    pub fn alternate_group(mut self, group: i16) -> Self {
        self.track.alternate_group = group;
        self
    }

    /// Sets the edit list for the track and returns the updated `TrackBuilder`.
    pub fn edit_list(mut self, segments: Vec<EditSegment>) -> Self {
        self.track.edit_list = Some(segments);
        self
    }

    /// Sets the default sample duration for the track and returns the updated `TrackBuilder`.
    pub fn sample_default_duration(mut self, duration: u32) -> Self {
        self.track.defaults.sample_duration = Some(duration);
        self
    }

    /// Sets the default sample size for the track and returns the updated `TrackBuilder`.
    pub fn sample_default_size(mut self, size: u32) -> Self {
        self.track.defaults.sample_size = Some(size);
        self
    }

    /// Sets the default sample flags for the track and returns the updated `TrackBuilder`.
    pub fn sample_default_flags(mut self, flags: SampleFlags) -> Self {
        self.track.defaults.sample_flags = Some(flags);
        self
    }

    /// Finalizes the track and adds it to the builder, returning the assigned track ID.
    pub fn build(self) -> u32 {
        let track_id = self.track.id.get();
        self.builder.movie.tracks.push(self.track);
        track_id
    }
}

/// A muxer for producing non-fragmented MP4 files.
#[derive(Debug)]
pub struct Muxer {
    movie: Movie,
}

impl Muxer {
    /// Creates a new [`Builder`] with the specified movie timescale.
    pub fn builder(timescale: u32) -> Result<Builder> {
        Builder::new(timescale)
    }

    /// Appends a chunk of samples to the muxer at the given byte offset within the `mdat` box.
    pub fn push_chunk<T: AsRef<[u8]>>(
        &mut self,
        samples: &[Sample<T>],
        data_offset: u64,
    ) -> Result<()> {
        let (track_id, chunk) = validate_and_build_chunk(samples, data_offset)?;
        self.movie.push_chunk(track_id.get(), chunk)
    }

    /// Finalizes the muxer and produces the `moov` box.
    pub fn finalize(&self) -> Result<MoovBox> {
        build_moov(&self.movie)
    }
}

/// A muxer for producing fragmented MP4 (fMP4) files.
#[derive(Debug)]
pub struct FragmentedMuxer {
    movie: Movie,
    sequence_number: u32,
    mdat_payload_size: u64,
}

impl FragmentedMuxer {
    /// Creates a new [`Builder`] with the specified movie timescale.
    pub fn builder(timescale: u32) -> Result<Builder> {
        Builder::new(timescale)
    }

    /// Appends a chunk of samples to the current fragment.
    ///
    /// The data offset within the `mdat` box is tracked automatically.
    pub fn push_chunk<T: AsRef<[u8]>>(&mut self, samples: &[Sample<T>]) -> Result<()> {
        let data_offset = self.mdat_payload_size;
        let (track_id, chunk) = validate_and_build_chunk(samples, data_offset)?;

        let total_size = chunk.total_size();
        let new_payload_size =
            self.mdat_payload_size
                .checked_add(total_size)
                .filter(|&size| size <= i32::MAX as u64)
                .ok_or(Error::new(ErrorKind::Overflow).with_message(
                    "mdat payload size exceeds i32 range; flush the fragment first",
                ))?;
        self.movie.push_chunk(track_id.get(), chunk)?;
        self.mdat_payload_size = new_payload_size;

        Ok(())
    }

    /// Flushes the accumulated samples into a `moof` box and resets the fragment state.
    ///
    /// The returned `MoofBox` has its `data_offset` values adjusted to account for
    /// the `moof` box size and the `mdat` header that immediately follows it.
    pub fn flush_fragment(&mut self) -> Result<MoofBox> {
        self.sequence_number = self
            .sequence_number
            .checked_add(1)
            .ok_or(ErrorKind::Overflow)?;

        let mut moof = build_moof(&self.movie, self.sequence_number, 0)?;
        let moof_boxed_size = moof.boxed_len();
        let mdat_header = BoxHeader::new(BoxType::MDAT, self.mdat_payload_size);

        let base = i32::try_from(moof_boxed_size + mdat_header.header_len()).map_err(|_| {
            Error::new(ErrorKind::Overflow)
                .with_message("moof + mdat header size exceeds i32 range for data_offset")
        })?;
        for traf in &mut moof.trafs {
            for trun in &mut traf.truns {
                if let Some(offset) = trun.data_offset() {
                    let adjusted = offset.checked_add(base).ok_or(
                        Error::new(ErrorKind::Overflow)
                            .with_message("data_offset + base exceeds i32 range"),
                    )?;
                    trun.set_data_offset(adjusted);
                }
            }
        }

        self.clear();
        Ok(moof)
    }

    fn clear(&mut self) {
        for track in self.movie.tracks.iter_mut() {
            track.sample_table.chunks.clear();
        }
        self.mdat_payload_size = 0;
    }
}

fn validate_and_build_chunk<T: AsRef<[u8]>>(
    samples: &[Sample<T>],
    data_offset: u64,
) -> Result<(NonZeroU32, SampleChunk)> {
    let first = samples
        .first()
        .ok_or(Error::new(ErrorKind::InvalidInput).with_message("samples must not be empty"))?;
    let track_id = first.track_id;

    let mut entries = Vec::with_capacity(samples.len());

    for sample in samples.iter() {
        if sample.track_id != track_id {
            return Err(Error::new(ErrorKind::InvalidInput)
                .with_message("All samples in a chunk must have the same track ID"));
        }

        entries.push(SampleMetadata::try_from(sample)?);
    }

    let chunk = SampleChunk {
        data_offset,
        entries,
    };

    Ok((track_id, chunk))
}

fn build_mvhd(movie: &Movie) -> Result<MvhdBox> {
    let mut mvhd = MvhdBox::default();
    mvhd.timescale = movie.timescale.get();
    mvhd.duration = duration_to_ticks(movie.movie_duration(), movie.timescale.get())
        .ok_or(ErrorKind::Overflow)?;
    mvhd.next_track_id = movie.next_track_id().ok_or(ErrorKind::Overflow)?;
    Ok(mvhd)
}

fn build_mvex(movie: &Movie) -> Result<MvexBox> {
    let mut trexs = Vec::with_capacity(movie.tracks.len());
    for track in movie.tracks.iter() {
        let trex = build_trex(track);
        trexs.push(trex);
    }

    Ok(MvexBox { mehd: None, trexs })
}

fn build_moov(movie: &Movie) -> Result<MoovBox> {
    let mvhd = build_mvhd(movie)?;
    let mut traks = Vec::with_capacity(movie.tracks.len());
    for track in movie.tracks.iter() {
        let trak = build_trak(track, movie.timescale)?;
        traks.push(trak);
    }

    Ok(MoovBox {
        mvhd,
        traks,
        mvex: None,
    })
}

fn build_moof(movie: &Movie, sequence_number: u32, moof_offset: u64) -> Result<MoofBox> {
    let mfhd = MfhdBox::new(sequence_number);
    let mut trafs = Vec::with_capacity(movie.tracks.len());
    for track in movie.tracks.iter() {
        if track.sample_table.chunks.is_empty() {
            continue; // Skip tracks with no samples for this fragment
        }
        let traf = build_traf(track, moof_offset)?;
        trafs.push(traf);
    }

    if trafs.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput)
            .with_message("Cannot build moof: no tracks have samples for this fragment"));
    }

    Ok(MoofBox { mfhd, trafs })
}

fn build_tkhd(track: &Track, movie_timescale: NonZeroU32) -> Result<TkhdBox> {
    let track_duration = track
        .edit_duration()
        .unwrap_or(track.sample_table.media_duration());
    let duration_ticks =
        duration_to_ticks(track_duration, movie_timescale.get()).ok_or(ErrorKind::Overflow)?;

    let mut tkhd = TkhdBox::new(track.id, duration_ticks);
    tkhd.alternate_group = track.alternate_group;
    tkhd.matrix = track.matrix;

    match &track.media {
        MediaDefinition::Video(desc) => {
            tkhd.width = U16F16::from_integer(i128::from(desc.width()));
            tkhd.height = U16F16::from_integer(i128::from(desc.height()));
        }
        MediaDefinition::Audio(_) => {
            tkhd.volume = U8F8::from_f32(1.0); // Full volume for audio tracks
        }
        _ => {}
    }

    Ok(tkhd)
}

fn build_mdhd(track: &Track) -> Result<MdhdBox> {
    let media_duration = track.sample_table.media_duration();
    let duration_ticks =
        duration_to_ticks(media_duration, track.timescale.get()).ok_or(ErrorKind::Overflow)?;

    let mdhd = MdhdBox::new(track.timescale, duration_ticks, track.language);
    Ok(mdhd)
}

fn build_stsd(track: &Track) -> Result<StsdBox> {
    let mut entries = Vec::with_capacity(1);

    let entry = match &track.media {
        MediaDefinition::Video(video_desc) => match video_desc {
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc1(entry) => {
                let payload = entry.encode_to_vec().map_err(Error::box_encode)?;
                RawBoxOwned::new(entry.boxtype(), payload)
            }
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc3(entry) => {
                let payload = entry.encode_to_vec().map_err(Error::box_encode)?;
                RawBoxOwned::new(entry.boxtype(), payload)
            }
            #[cfg(feature = "mp4")]
            VisualSampleDescription::Mp4v(entry) => {
                let payload = entry.encode_to_vec().map_err(Error::box_encode)?;
                RawBoxOwned::new(entry.boxtype(), payload)
            }
        },
        MediaDefinition::Audio(audio_desc) => match audio_desc {
            #[cfg(feature = "mp4")]
            AudioSampleDescription::Mp4a(entry) => {
                let payload = entry.encode_to_vec().map_err(Error::box_encode)?;
                RawBoxOwned::new(entry.boxtype(), payload)
            }
        },
        MediaDefinition::Metadata(_)
        | MediaDefinition::Hint(_)
        | MediaDefinition::Text(_)
        | MediaDefinition::Subtitle(_)
        | MediaDefinition::Font(_)
        | MediaDefinition::Other(_) => {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(
                "Sample description encoding is only supported for video and audio media types",
            ));
        }
    };

    entries.push(entry);

    Ok(StsdBox {
        entries,
        ..Default::default()
    })
}

fn build_media_header(track: &Track) -> MediaHeaderBox {
    match &track.media {
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
        MediaDefinition::Subtitle(_) | MediaDefinition::Other(_) => {
            let nmhd = NmhdBox::default();
            MediaHeaderBox::Nmhd(nmhd)
        }
    }
}

fn build_minf(track: &Track) -> Result<MinfBox> {
    let media_header = build_media_header(track);
    let stsd = build_stsd(track)?;
    let stbl = build_stbl(&track.sample_table, stsd, track.timescale)?;
    let dinf = DinfBox::self_contained();

    Ok(MinfBox {
        media_header,
        dinf,
        stbl,
    })
}

fn build_mdia(track: &Track) -> Result<MdiaBox> {
    let mdhd = build_mdhd(track)?;
    let hdlr = HdlrBox::new(track.media.handler_type());
    let minf = build_minf(track)?;

    Ok(MdiaBox {
        mdhd,
        hdlr,
        minf,
        elng: None,
    })
}

fn build_elst(track: &Track, movie_timescale: NonZeroU32) -> Result<Option<ElstBox>> {
    let Some(segments) = &track.edit_list else {
        return Ok(None);
    };

    let media_timescale = track.timescale;

    let mut entries = Vec::with_capacity(segments.len());

    for seg in segments.iter() {
        let entry = match seg {
            EditSegment::Empty { duration } => {
                let segment_duration = duration_to_ticks(*duration, movie_timescale.get())
                    .ok_or(ErrorKind::Overflow)?;
                ElstEntry {
                    segment_duration,
                    media_time: -1,
                    media_rate: I16F16::from_f32(1.0),
                }
            }
            EditSegment::Media {
                duration,
                media_start,
                media_rate,
            } => {
                let segment_duration = duration_to_ticks(*duration, movie_timescale.get())
                    .ok_or(ErrorKind::Overflow)?;
                let media_time = duration_to_ticks(*media_start, media_timescale.get())
                    .ok_or(ErrorKind::Overflow)?;
                ElstEntry {
                    segment_duration,
                    media_time: i64::try_from(media_time)?,
                    media_rate: I16F16::from_f32(*media_rate),
                }
            }
            EditSegment::Dwell {
                duration,
                media_time,
            } => {
                let segment_duration = duration_to_ticks(*duration, movie_timescale.get())
                    .ok_or(ErrorKind::Overflow)?;
                let media_time = duration_to_ticks(*media_time, media_timescale.get())
                    .ok_or(ErrorKind::Overflow)?;
                ElstEntry {
                    segment_duration,
                    media_time: i64::try_from(media_time)?,
                    media_rate: I16F16::from_f32(0.0),
                }
            }
        };
        entries.push(entry);
    }

    Ok(Some(ElstBox::new(entries)))
}

fn build_edts(track: &Track, movie_timescale: NonZeroU32) -> Result<Option<EdtsBox>> {
    let elst = build_elst(track, movie_timescale)?;

    match elst {
        Some(elst) => Ok(Some(EdtsBox { elst: Some(elst) })),
        None => Ok(None),
    }
}

fn build_trak(track: &Track, movie_timescale: NonZeroU32) -> Result<TrakBox> {
    let tkhd = build_tkhd(track, movie_timescale)?;
    let mdia = build_mdia(track)?;
    let edts = build_edts(track, movie_timescale)?;

    Ok(TrakBox {
        tkhd,
        tref: None,
        trgr: None,
        mdia,
        edts,
    })
}

fn build_trex(track: &Track) -> TrexBox {
    TrexBox {
        track_id: track.id.get(),
        default_sample_description_index: 1,
        default_sample_duration: track.defaults.sample_duration.unwrap_or_default(),
        default_sample_size: track.defaults.sample_size.unwrap_or_default(),
        default_sample_flags: track.defaults.sample_flags.unwrap_or_default(),
        ..Default::default()
    }
}

fn build_traf(track: &Track, moof_offset: u64) -> Result<TrafBox> {
    let flags = TfhdFlags::DEFAULT_BASE_IS_MOOF
        | if track.defaults.sample_duration.is_some() {
            TfhdFlags::DEFAULT_SAMPLE_DURATION_PRESENT
        } else {
            TfhdFlags::empty()
        }
        | if track.defaults.sample_size.is_some() {
            TfhdFlags::DEFAULT_SAMPLE_SIZE_PRESENT
        } else {
            TfhdFlags::empty()
        }
        | if track.defaults.sample_flags.is_some() {
            TfhdFlags::DEFAULT_SAMPLE_FLAGS_PRESENT
        } else {
            TfhdFlags::empty()
        };

    let tfhd = TfhdBox {
        flags,
        track_id: track.id.get(),
        ..Default::default()
    };

    let first_decode_time_ns = track.sample_table.first_decode_time_ns().ok_or(
        Error::new(ErrorKind::InvalidInput)
            .with_message("Track must have at least one sample to build a traf box"),
    )?;

    let base_media_decode_time =
        nanos_to_ticks(first_decode_time_ns, track.timescale.get()).ok_or(ErrorKind::Overflow)?;

    let tfdt = TfdtBox::new(base_media_decode_time);

    let truns = build_truns(
        &track.sample_table,
        track.timescale,
        moof_offset,
        track.defaults.sample_duration,
        track.defaults.sample_size,
        track.defaults.sample_flags,
    )?;

    Ok(TrafBox {
        tfhd,
        tfdt: Some(tfdt),
        truns,
        sbgps: Vec::new(),
        sgpds: Vec::new(),
    })
}

fn build_stbl(
    sample_table: &SampleTable,
    stsd: StsdBox,
    media_timescale: NonZeroU32,
) -> Result<StblBox> {
    let mut stts = SttsBox::default();
    let mut stsz = StszBox::default();
    let mut stsc = StscBox::default();
    let mut ctts = CttsBox::default();
    let mut stss = StssBox::default();
    let mut chunk_offset = ChunkOffset::default();

    let mut has_nonzero_ctts = false;
    let mut chunk_number: u32 = 0;
    let mut sample_number: u32 = 0;

    for chunk in sample_table.chunks.iter() {
        debug_assert!(!chunk.entries.is_empty());

        chunk_number = chunk_number.checked_add(1).ok_or(ErrorKind::Overflow)?;
        let samples_per_chunk = u32::try_from(chunk.entries.len()).map_err(|_| {
            Error::new(ErrorKind::Overflow)
                .with_message("Number of samples per chunk exceeds u32 range")
        })?;
        stsc.push(chunk_number, samples_per_chunk, 1);
        chunk_offset.push(chunk.data_offset);

        for sample in chunk.entries.iter() {
            sample_number = sample_number.checked_add(1).ok_or(ErrorKind::Overflow)?;

            let sample_delta = sample.sample_delta(media_timescale)?;
            let cto = sample.composition_time_offset(media_timescale)?;
            stts.push(sample_delta);
            stsz.push(sample.size);
            ctts.push(cto);
            has_nonzero_ctts |= cto != 0;

            if sample.is_sync {
                stss.push(sample_number);
            }
        }
    }

    let sample_size = SampleSize::Stsz(stsz);
    let ctts = if has_nonzero_ctts { Some(ctts) } else { None };
    let stss = if stss.entries.len() == sample_number as usize {
        None
    } else {
        Some(stss)
    };

    Ok(StblBox {
        stsd,
        stts,
        sample_size,
        stsc,
        chunk_offset,
        stdp: None,
        ctts,
        cslg: None,
        stss,
        stsh: None,
        sdtp: None,
        sbgps: Vec::new(),
        sgpds: Vec::new(),
    })
}

/// Strategy for emitting sample flags in a `trun` box.
enum FlagsStrategy {
    /// All samples match the default flags — omit flags entirely.
    AllDefault,
    /// Only the first sample differs from the default — use `first_sample_flags`.
    FirstSampleOnly(SampleFlags),
    /// Multiple samples differ — write per-sample flags.
    PerSample,
}

fn build_trun(
    chunk: &SampleChunk,
    media_timescale: NonZeroU32,
    moof_offset: u64,
    default_sample_duration: Option<u32>,
    default_sample_size: Option<u32>,
    default_sample_flags: Option<SampleFlags>,
) -> Result<TrunBox> {
    if chunk.entries.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput)
            .with_message("Cannot build trun from an empty chunk"));
    }

    // 1. Convert all samples to timescale units
    let computed: Vec<_> = chunk
        .entries
        .iter()
        .map(|meta| {
            Ok((
                meta.sample_delta(media_timescale)?,
                meta.size,
                meta.sample_flags(),
                meta.composition_time_offset(media_timescale)?,
            ))
        })
        .collect::<Result<_>>()?;

    // 2. Decide which per-sample fields to emit
    let emit_duration = match default_sample_duration {
        Some(d) if computed.iter().all(|&(dur, ..)| dur == d) => false,
        _ => true,
    };

    let emit_size = match default_sample_size {
        Some(s) if computed.iter().all(|&(_, size, ..)| size == s) => false,
        _ => true,
    };

    let has_cto = computed.iter().any(|&(.., cto)| cto != 0);
    let signed_cto = computed.iter().any(|&(.., cto)| cto < 0);

    // Determine how to emit sample flags:
    //  - AllDefault:       all samples match default → omit flags entirely
    //  - FirstSampleOnly:  only sample[0] differs   → use first_sample_flags field
    //  - PerSample:        multiple differ           → write flags per sample
    let flags_strategy = match default_sample_flags {
        Some(df) if computed.iter().all(|c| c.2 == df) => FlagsStrategy::AllDefault,
        Some(df) if computed[0].2 != df && computed[1..].iter().all(|c| c.2 == df) => {
            FlagsStrategy::FirstSampleOnly(computed[0].2)
        }
        _ => FlagsStrategy::PerSample,
    };

    // 3. Build flags
    let mut trun_flags = TrunFlags::DATA_OFFSET_PRESENT;
    if emit_duration {
        trun_flags |= TrunFlags::SAMPLE_DURATION_PRESENT;
    }
    if emit_size {
        trun_flags |= TrunFlags::SAMPLE_SIZE_PRESENT;
    }
    match flags_strategy {
        FlagsStrategy::AllDefault => {}
        FlagsStrategy::FirstSampleOnly(_) => {
            trun_flags |= TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT;
        }
        FlagsStrategy::PerSample => {
            trun_flags |= TrunFlags::SAMPLE_FLAGS_PRESENT;
        }
    }
    if has_cto {
        trun_flags |= TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT;
    }

    // 4. Populate TrunBox
    let mut trun = TrunBox::new(trun_flags, signed_cto);

    let relative_offset = chunk
        .data_offset
        .checked_sub(moof_offset)
        .ok_or(ErrorKind::Overflow)?;
    trun.set_data_offset(i32::try_from(relative_offset)?);

    if let FlagsStrategy::FirstSampleOnly(fsf) = flags_strategy {
        trun.set_first_sample_flags(fsf);
    }

    let emit_flags = matches!(flags_strategy, FlagsStrategy::PerSample);
    for &(duration, size, flags, cto) in &computed {
        trun.push_entry(TrunEntry {
            duration: emit_duration.then_some(duration),
            size: emit_size.then_some(size),
            flags: emit_flags.then_some(flags),
            composition_time_offset: has_cto.then(|| i64::from(cto)),
        });
    }

    Ok(trun)
}

fn build_truns(
    sample_table: &SampleTable,
    media_timescale: NonZeroU32,
    moof_offset: u64,
    default_sample_duration: Option<u32>,
    default_sample_size: Option<u32>,
    default_sample_flags: Option<SampleFlags>,
) -> Result<Vec<TrunBox>> {
    let mut truns = Vec::with_capacity(sample_table.chunks.len());

    for chunk in sample_table.chunks.iter() {
        let trun = build_trun(
            chunk,
            media_timescale,
            moof_offset,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        )?;
        truns.push(trun);
    }

    Ok(truns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    const TS_90K: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(90_000) };

    fn sample_meta(dts_ns: u64, duration_ms: u64, size: u32, is_sync: bool) -> SampleMetadata {
        SampleMetadata {
            dts_ns,
            pts_ns: None,
            duration: Duration::from_millis(duration_ms),
            is_sync,
            size,
        }
    }

    fn chunk_from(entries: Vec<SampleMetadata>, data_offset: u64) -> SampleChunk {
        SampleChunk {
            data_offset,
            entries,
        }
    }

    // ---------------------------------------------------------------
    // validate_and_build_chunk
    // ---------------------------------------------------------------

    #[test]
    fn validate_empty_samples_returns_error() {
        let samples: &[Sample<&[u8]>] = &[];
        let err = validate_and_build_chunk(samples, 0).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn validate_mixed_track_ids_returns_error() {
        let s1 = Sample::new(1, 0, None, Duration::from_millis(33), true, &[0u8; 100][..]);
        let s2 = Sample::new(2, 33_000_000, None, Duration::from_millis(33), false, &[0u8; 100][..]);
        let err = validate_and_build_chunk(&[s1, s2], 0).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn validate_single_sample_ok() {
        let s = Sample::new(1, 0, None, Duration::from_millis(33), true, &[0u8; 50][..]);
        let (track_id, chunk) = validate_and_build_chunk(&[s], 128).unwrap();
        assert_eq!(track_id.get(), 1);
        assert_eq!(chunk.entries.len(), 1);
        assert_eq!(chunk.data_offset, 128);
        assert_eq!(chunk.entries[0].size, 50);
    }

    // ---------------------------------------------------------------
    // build_trun: FlagsStrategy
    // ---------------------------------------------------------------

    #[test]
    fn trun_flags_all_default() {
        let df = SampleFlags::non_sync();
        let entries = vec![
            sample_meta(0, 33, 1000, false),
            sample_meta(33_000_000, 33, 1000, false),
        ];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, None, Some(df)).unwrap();

        // flags should not contain SAMPLE_FLAGS_PRESENT or FIRST_SAMPLE_FLAGS_PRESENT
        assert!(!trun.flags().contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
        assert!(!trun.flags().contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
        assert!(trun.sample_flags().is_none());
    }

    #[test]
    fn trun_flags_first_sample_only() {
        let df = SampleFlags::non_sync();
        let entries = vec![
            sample_meta(0, 33, 1000, true),               // sync → differs from default
            sample_meta(33_000_000, 33, 1000, false),      // non_sync → matches default
            sample_meta(66_000_000, 33, 1000, false),      // non_sync → matches default
        ];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, None, Some(df)).unwrap();

        assert!(trun.flags().contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
        assert!(!trun.flags().contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
        assert_eq!(trun.first_sample_flags(), Some(SampleFlags::sync()));
    }

    #[test]
    fn trun_flags_per_sample() {
        let df = SampleFlags::non_sync();
        let entries = vec![
            sample_meta(0, 33, 1000, true),
            sample_meta(33_000_000, 33, 1000, true),       // second also differs
            sample_meta(66_000_000, 33, 1000, false),
        ];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, None, Some(df)).unwrap();

        assert!(trun.flags().contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
        assert!(!trun.flags().contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
        assert_eq!(trun.sample_flags().unwrap().len(), 3);
    }

    #[test]
    fn trun_flags_no_default_uses_per_sample() {
        let entries = vec![
            sample_meta(0, 33, 1000, true),
            sample_meta(33_000_000, 33, 1000, true),
        ];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, None, None).unwrap();

        assert!(trun.flags().contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
    }

    #[test]
    fn trun_single_sample_all_default() {
        let df = SampleFlags::sync();
        let entries = vec![sample_meta(0, 33, 1000, true)];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, None, Some(df)).unwrap();

        assert!(!trun.flags().contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
        assert!(!trun.flags().contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
        assert_eq!(trun.sample_count(), 1);
    }

    #[test]
    fn trun_single_sample_differs_uses_first_sample_flags() {
        let df = SampleFlags::non_sync();
        let entries = vec![sample_meta(0, 33, 1000, true)];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, None, Some(df)).unwrap();

        assert!(trun.flags().contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
        assert_eq!(trun.first_sample_flags(), Some(SampleFlags::sync()));
    }

    #[test]
    fn trun_empty_chunk_returns_error() {
        let chunk = chunk_from(vec![], 0);
        let err = build_trun(&chunk, TS_90K, 0, None, None, None).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    // ---------------------------------------------------------------
    // build_trun: duration/size omission
    // ---------------------------------------------------------------

    #[test]
    fn trun_omits_duration_when_all_match_default() {
        let entries = vec![
            sample_meta(0, 33, 500, true),
            sample_meta(33_000_000, 33, 600, true),
        ];
        let chunk = chunk_from(entries, 0);
        // 33ms * 90000 = 2970
        let trun = build_trun(&chunk, TS_90K, 0, Some(2970), None, None).unwrap();

        assert!(!trun.flags().contains(TrunFlags::SAMPLE_DURATION_PRESENT));
        assert!(trun.sample_durations().is_none());
    }

    #[test]
    fn trun_omits_size_when_all_match_default() {
        let entries = vec![
            sample_meta(0, 33, 1000, true),
            sample_meta(33_000_000, 33, 1000, true),
        ];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, Some(1000), None).unwrap();

        assert!(!trun.flags().contains(TrunFlags::SAMPLE_SIZE_PRESENT));
        assert!(trun.sample_sizes().is_none());
    }

    #[test]
    fn trun_emits_size_when_mismatch() {
        let entries = vec![
            sample_meta(0, 33, 1000, true),
            sample_meta(33_000_000, 33, 2000, true),
        ];
        let chunk = chunk_from(entries, 0);

        let trun = build_trun(&chunk, TS_90K, 0, None, Some(1000), None).unwrap();

        assert!(trun.flags().contains(TrunFlags::SAMPLE_SIZE_PRESENT));
        assert_eq!(trun.sample_sizes().unwrap(), &[1000, 2000]);
    }

    // ---------------------------------------------------------------
    // build_stbl: ctts / stss
    // ---------------------------------------------------------------

    #[test]
    fn stbl_ctts_none_when_all_zero() {
        let entries = vec![
            sample_meta(0, 33, 100, true),
            sample_meta(33_000_000, 33, 100, true),
        ];
        let table = SampleTable {
            chunks: vec![chunk_from(entries, 0)],
        };
        let stsd = StsdBox::default();
        let stbl = build_stbl(&table, stsd, TS_90K).unwrap();

        assert!(stbl.ctts.is_none());
    }

    #[test]
    fn stbl_ctts_present_when_nonzero() {
        let entries = vec![
            SampleMetadata {
                dts_ns: 0,
                pts_ns: Some(33_000_000),
                duration: Duration::from_millis(33),
                is_sync: true,
                size: 100,
            },
            sample_meta(33_000_000, 33, 100, false),
        ];
        let table = SampleTable {
            chunks: vec![chunk_from(entries, 0)],
        };
        let stsd = StsdBox::default();
        let stbl = build_stbl(&table, stsd, TS_90K).unwrap();

        assert!(stbl.ctts.is_some());
    }

    #[test]
    fn stbl_stss_none_when_all_sync() {
        let entries = vec![
            sample_meta(0, 33, 100, true),
            sample_meta(33_000_000, 33, 100, true),
        ];
        let table = SampleTable {
            chunks: vec![chunk_from(entries, 0)],
        };
        let stsd = StsdBox::default();
        let stbl = build_stbl(&table, stsd, TS_90K).unwrap();

        assert!(stbl.stss.is_none());
    }

    #[test]
    fn stbl_stss_present_when_partial_sync() {
        let entries = vec![
            sample_meta(0, 33, 100, true),
            sample_meta(33_000_000, 33, 100, false),
            sample_meta(66_000_000, 33, 100, true),
        ];
        let table = SampleTable {
            chunks: vec![chunk_from(entries, 0)],
        };
        let stsd = StsdBox::default();
        let stbl = build_stbl(&table, stsd, TS_90K).unwrap();

        let stss = stbl.stss.unwrap();
        assert_eq!(stss.entries.len(), 2);
        assert_eq!(stss.entries[0].sample_number, 1);
        assert_eq!(stss.entries[1].sample_number, 3);
    }

    // ---------------------------------------------------------------
    // Helper: minimal MediaDefinition for public API tests
    // ---------------------------------------------------------------

    #[cfg(feature = "avc")]
    fn test_video_media(width: u16, height: u16) -> MediaDefinition {
        use mp4_bmff::boxes::avc::{Avc1SampleEntry, AvcCBox};
        use mp4_bmff::boxes::bmff::VisualSampleEntry;
        use mp4_bmff::formats::mpeg4::codecs::avc::AVCDecoderConfigurationRecord;

        let avcc = AvcCBox {
            avc_config: AVCDecoderConfigurationRecord {
                configuration_version: 1,
                avc_profile_indication: 66,
                profile_compatibility: 0xC0,
                avc_level_indication: 30,
                length_size_minus_one: 3,
                sps: vec![vec![0x67, 0x42, 0xC0, 0x1E]],
                pps: vec![vec![0x68, 0xCE, 0x3C, 0x80]],
                chroma_format: None,
                bit_depth_luma_minus8: None,
                bit_depth_chroma_minus8: None,
                sps_ext: None,
            },
        };
        let mut base = VisualSampleEntry::default();
        base.width = width;
        base.height = height;
        let entry = Avc1SampleEntry::new(base, avcc, None, None);
        MediaDefinition::Video(VisualSampleDescription::Avc1(entry))
    }

    // ---------------------------------------------------------------
    // Muxer public API
    // ---------------------------------------------------------------

    #[cfg(feature = "avc")]
    #[test]
    fn muxer_finalize_produces_valid_moov() {
        let mut builder = Muxer::builder(1000).unwrap();
        let track_id = builder
            .add_track(90_000, test_video_media(1280, 720))
            .unwrap()
            .build();
        let mut muxer = builder.build().unwrap();

        let samples = [
            Sample::new(track_id, 0, None, Duration::from_millis(33), true, vec![0u8; 1000]),
            Sample::new(track_id, 33_000_000, None, Duration::from_millis(33), false, vec![0u8; 800]),
            Sample::new(track_id, 66_000_000, None, Duration::from_millis(33), true, vec![0u8; 900]),
        ];
        muxer.push_chunk(&samples, 0).unwrap();

        let moov = muxer.finalize().unwrap();

        assert_eq!(moov.traks.len(), 1);
        assert!(moov.mvex.is_none());
        assert_eq!(moov.mvhd.timescale, 1000);
        assert_eq!(moov.mvhd.next_track_id, 2);

        let trak = &moov.traks[0];
        assert_eq!(trak.tkhd.track_id, track_id);

        let stbl = &trak.mdia.minf.stbl;
        assert_eq!(stbl.stts.entries.len(), 1); // all same duration → single run
        assert!(stbl.stss.is_some()); // not all sync
        assert_eq!(stbl.stss.as_ref().unwrap().entries.len(), 2); // samples 1 and 3
    }

    #[cfg(feature = "avc")]
    #[test]
    fn muxer_push_chunk_validates_track_id() {
        let mut builder = Muxer::builder(1000).unwrap();
        builder
            .add_track(90_000, test_video_media(1920, 1080))
            .unwrap()
            .build();
        let mut muxer = builder.build().unwrap();

        // track_id 99 does not exist
        let sample = Sample::new(99, 0, None, Duration::from_millis(33), true, vec![0u8; 100]);
        let err = muxer.push_chunk(&[sample], 0).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    // ---------------------------------------------------------------
    // FragmentedMuxer public API
    // ---------------------------------------------------------------

    #[cfg(feature = "avc")]
    #[test]
    fn fragmented_muxer_produces_moov_with_mvex() {
        let mut builder = FragmentedMuxer::builder(1000).unwrap();
        builder
            .add_track(90_000, test_video_media(1280, 720))
            .unwrap()
            .sample_default_duration(3000)
            .sample_default_flags(SampleFlags::non_sync())
            .build();
        let (moov, _muxer) = builder.build_fragmented().unwrap();

        assert!(moov.mvex.is_some());
        let mvex = moov.mvex.unwrap();
        assert_eq!(mvex.trexs.len(), 1);
        assert_eq!(mvex.trexs[0].default_sample_duration, 3000);
    }

    #[cfg(feature = "avc")]
    #[test]
    fn fragmented_muxer_flush_produces_moof() {
        let mut builder = FragmentedMuxer::builder(1000).unwrap();
        let track_id = builder
            .add_track(90_000, test_video_media(1280, 720))
            .unwrap()
            .sample_default_duration(3000)
            .sample_default_flags(SampleFlags::non_sync())
            .build();
        let (_moov, mut muxer) = builder.build_fragmented().unwrap();

        let samples = [
            Sample::new(track_id, 0, None, Duration::from_millis(33), true, vec![0u8; 500]),
            Sample::new(track_id, 33_000_000, None, Duration::from_millis(33), false, vec![0u8; 400]),
        ];
        muxer.push_chunk(&samples).unwrap();

        let moof = muxer.flush_fragment().unwrap();

        assert_eq!(moof.mfhd.sequence_number, 1);
        assert_eq!(moof.trafs.len(), 1);

        let traf = &moof.trafs[0];
        assert!(traf.tfdt.is_some());
        assert_eq!(traf.truns.len(), 1);
        assert_eq!(traf.truns[0].sample_count(), 2);

        // data_offset should be positive (moof_size + mdat_header)
        let offset = traf.truns[0].data_offset().unwrap();
        assert!(offset > 0);
    }

    #[cfg(feature = "avc")]
    #[test]
    fn fragmented_muxer_flush_resets_state() {
        let mut builder = FragmentedMuxer::builder(1000).unwrap();
        let track_id = builder
            .add_track(90_000, test_video_media(1280, 720))
            .unwrap()
            .build();
        let (_moov, mut muxer) = builder.build_fragmented().unwrap();

        // First fragment
        let s1 = [Sample::new(track_id, 0, None, Duration::from_millis(33), true, vec![0u8; 100])];
        muxer.push_chunk(&s1).unwrap();
        let moof1 = muxer.flush_fragment().unwrap();
        assert_eq!(moof1.mfhd.sequence_number, 1);

        // Second fragment — sequence number increments, data_offset resets
        let s2 = [Sample::new(track_id, 33_000_000, None, Duration::from_millis(33), true, vec![0u8; 200])];
        muxer.push_chunk(&s2).unwrap();
        let moof2 = muxer.flush_fragment().unwrap();
        assert_eq!(moof2.mfhd.sequence_number, 2);
        assert_eq!(moof2.trafs[0].truns[0].sample_count(), 1);
    }

    #[cfg(feature = "avc")]
    #[test]
    fn fragmented_muxer_flush_empty_returns_error() {
        let mut builder = FragmentedMuxer::builder(1000).unwrap();
        builder
            .add_track(90_000, test_video_media(1280, 720))
            .unwrap()
            .build();
        let (_moov, mut muxer) = builder.build_fragmented().unwrap();

        let err = muxer.flush_fragment().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}
