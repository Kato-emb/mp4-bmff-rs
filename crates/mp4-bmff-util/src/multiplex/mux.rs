use core::num::NonZeroU32;

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
        let moov = build_moov(&self.movie, true)?;

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

#[derive(Debug)]
pub struct Muxer {
    movie: Movie,
}

impl Muxer {
    pub fn builder(timescale: u32) -> Result<Builder> {
        Builder::new(timescale)
    }

    pub fn push_chunk<T: AsRef<[u8]>>(
        &mut self,
        samples: &[Sample<T>],
        data_offset: u64,
    ) -> Result<()> {
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

        self.movie.push_chunk(track_id.get(), chunk)
    }

    pub fn finalize(&self) -> Result<MoovBox> {
        build_moov(&self.movie, false)
    }
}

#[derive(Debug)]
pub struct FragmentedMuxer {
    movie: Movie,
    sequence_number: u32,
    mdat_payload_size: u64,
}

impl FragmentedMuxer {
    pub fn builder(timescale: u32) -> Result<Builder> {
        Builder::new(timescale)
    }

    pub fn push_chunk<T: AsRef<[u8]>>(&mut self, samples: &[Sample<T>]) -> Result<()> {
        let data_offset = self.mdat_payload_size;

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

        let total_size = chunk.total_size();
        self.movie.push_chunk(track_id.get(), chunk)?;
        self.mdat_payload_size += total_size;

        Ok(())
    }

    pub fn flush_fragment(&mut self) -> Result<MoofBox> {
        self.sequence_number += 1;

        let mut moof = build_moof(&self.movie, self.sequence_number, 0)?;
        let moof_boxed_size = moof.boxed_len();
        let mdat_header = BoxHeader::new(BoxType::MDAT, self.mdat_payload_size);

        let base = i32::try_from(moof_boxed_size + mdat_header.header_len())?;
        for traf in &mut moof.trafs {
            for trun in &mut traf.truns {
                if let Some(offset) = trun.data_offset() {
                    trun.set_data_offset(offset + base);
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

fn build_moov(movie: &Movie, fragmented: bool) -> Result<MoovBox> {
    let mvhd = build_mvhd(movie)?;
    let mut traks = Vec::with_capacity(movie.tracks.len());
    for track in movie.tracks.iter() {
        let trak = build_trak(track, movie.timescale)?;
        traks.push(trak);
    }

    let mut moov = MoovBox {
        mvhd,
        traks,
        mvex: None,
    };

    if fragmented {
        debug_assert!(
            movie
                .tracks
                .iter()
                .all(|t| t.sample_table.chunks.is_empty()),
            "Fragmented movies should not have samples in the MovieBox; samples go in separate MovieFragment boxes (moof)"
        );

        let mvex = build_mvex(movie)?;
        moov.mvex = Some(mvex);
    }

    Ok(moov)
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
        _ => {
            unimplemented!("to_raw_box is only implemented for video and audio sample descriptions")
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
        stsc.push(chunk_number, chunk.entries.len() as u32, 1);
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
    debug_assert!(!chunk.entries.is_empty());

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
    let all_match_default =
        default_sample_flags.is_some_and(|df| computed.iter().all(|c| c.2 == df));
    let flags_strategy = if all_match_default {
        FlagsStrategy::AllDefault
    } else if let Some(df) = default_sample_flags {
        if computed.len() > 1 && computed[0].2 != df && computed[1..].iter().all(|c| c.2 == df) {
            FlagsStrategy::FirstSampleOnly(computed[0].2)
        } else {
            FlagsStrategy::PerSample
        }
    } else {
        FlagsStrategy::PerSample
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
