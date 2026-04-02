use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::*;
use mp4_bmff::types::*;

use super::{
    AudioSampleDescription, //
    EditSegment,
    Error,
    ErrorKind,
    MediaDefinition,
    Result,
    Sample,
    VisualSampleDescription,
};

use super::types::{
    Movie, //
    SampleTable,
    Track,
    TrackDefaults,
};

pub(super) fn build_moov(movie: &Movie) -> Result<MoovBox> {
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

fn build_mvhd(movie: &Movie) -> Result<MvhdBox> {
    let scale = TickScale(movie.timescale);
    let mut mvhd = MvhdBox::default();
    mvhd.timescale = movie.timescale.get();
    mvhd.duration = movie
        .tracks
        .iter()
        .map(|t| scale.track_duration(t))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    mvhd.next_track_id = movie.next_track_id().get();
    Ok(mvhd)
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

fn build_tkhd(track: &Track, movie_timescale: NonZeroU32) -> Result<TkhdBox> {
    let track_duration_ticks = TickScale(movie_timescale).track_duration(track)?;

    let mut tkhd = TkhdBox::new(track.id.into_inner(), track_duration_ticks);
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
        _ => unimplemented!("Unsupported media type"),
    }

    Ok(tkhd)
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

fn build_mdhd(track: &Track) -> Result<MdhdBox> {
    let media_duration_ticks = TickScale(track.timescale).media_duration(&track.sample_table)?;

    let mdhd = MdhdBox::new(track.timescale, media_duration_ticks, track.language);
    Ok(mdhd)
}

fn build_minf(track: &Track) -> Result<MinfBox> {
    let media_header = build_media_header(track);
    let stsd = build_stsd(track)?;
    let scale = TickScale(track.timescale);
    let stbl = build_stbl(&track.sample_table, stsd, &scale)?;
    let dinf = DinfBox::self_contained();

    Ok(MinfBox {
        media_header,
        dinf,
        stbl,
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
        MediaDefinition::Hint(_) => {
            let hmhd = HmhdBox::default();
            MediaHeaderBox::Hmhd(hmhd)
        }
        MediaDefinition::Metadata(_)
        | MediaDefinition::Text(_)
        | MediaDefinition::Font(_)
        | MediaDefinition::Subtitle(_)
        | MediaDefinition::Other(_) => {
            let nmhd = NmhdBox::default();
            MediaHeaderBox::Nmhd(nmhd)
        }
    }
}

fn encode_sample_entry(entry: &(impl BoxCodec + BoxEncode)) -> Result<RawBoxOwned> {
    let payload = entry.encode_to_vec().map_err(Error::box_encode)?;
    Ok(RawBoxOwned::new(entry.boxtype(), payload))
}

fn build_stsd(track: &Track) -> Result<StsdBox> {
    let entry = match &track.media {
        MediaDefinition::Video(video_desc) => match video_desc {
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc1(entry) => encode_sample_entry(entry)?,
            #[cfg(feature = "avc")]
            VisualSampleDescription::Avc3(entry) => encode_sample_entry(entry)?,
            #[cfg(feature = "mp4")]
            VisualSampleDescription::Mp4v(entry) => encode_sample_entry(entry)?,
        },
        MediaDefinition::Audio(audio_desc) => match audio_desc {
            #[cfg(feature = "mp4")]
            AudioSampleDescription::Mp4a(entry) => encode_sample_entry(entry)?,
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

    Ok(StsdBox {
        entries: alloc::vec![entry],
        ..Default::default()
    })
}

fn build_stbl(sample_table: &SampleTable, stsd: StsdBox, scale: &TickScale) -> Result<StblBox> {
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
        chunk_number = chunk_number.checked_add(1).ok_or(ErrorKind::Overflow)?;
        let samples_per_chunk = u32::try_from(chunk.sample_count()).map_err(|_| {
            Error::new(ErrorKind::Overflow)
                .with_message("Number of samples per chunk exceeds u32 range")
        })?;
        stsc.push(chunk_number, samples_per_chunk, 1);
        chunk_offset.push(chunk.data_offset());

        for sample in chunk.samples() {
            sample_number = sample_number.checked_add(1).ok_or(ErrorKind::Overflow)?;
            let delta_ticks = scale.sample_delta(sample)?;
            let cto_ticks = i32::try_from(scale.composition_time_offset(sample)?)?;

            stts.push(delta_ticks);
            stsz.push(sample.size);
            ctts.push(cto_ticks);
            has_nonzero_ctts |= cto_ticks != 0;

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

fn build_edts(track: &Track, movie_timescale: NonZeroU32) -> Result<Option<EdtsBox>> {
    let elst = build_elst(track, movie_timescale)?;
    Ok(elst.map(|elst| EdtsBox { elst: Some(elst) }))
}

fn build_elst(track: &Track, movie_timescale: NonZeroU32) -> Result<Option<ElstBox>> {
    let Some(segments) = &track.edit_list else {
        return Ok(None);
    };

    let movie_scale = TickScale(movie_timescale);
    let media_scale = TickScale(track.timescale);

    let mut entries = Vec::with_capacity(segments.len());

    for seg in segments.iter() {
        let entry = match seg {
            EditSegment::Empty { duration } => {
                let segment_duration = movie_scale
                    .from_duration(*duration)
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
                let segment_duration = movie_scale
                    .from_duration(*duration)
                    .ok_or(ErrorKind::Overflow)?;
                let media_time = media_scale
                    .from_duration(*media_start)
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
                let segment_duration = movie_scale
                    .from_duration(*duration)
                    .ok_or(ErrorKind::Overflow)?;
                let media_time = media_scale
                    .from_duration(*media_time)
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

pub(super) fn build_moof(movie: &Movie, sequence_number: u32) -> Result<MoofBox> {
    let mfhd = MfhdBox::new(sequence_number);
    let mut trafs = Vec::with_capacity(movie.tracks.len());
    for track in movie.tracks.iter() {
        if track.sample_table.chunks.is_empty() {
            continue; // Skip tracks with no samples for this fragment
        }
        let traf = build_traf(track)?;
        trafs.push(traf);
    }

    Ok(MoofBox { mfhd, trafs })
}

fn build_traf(track: &Track) -> Result<TrafBox> {
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

    let first_sample_dts = track.sample_table.first_sample_dts().ok_or(
        Error::new(ErrorKind::InvalidInput)
            .with_message("Track must have at least one sample to build a traf box"),
    )?;

    let scale = TickScale(track.timescale);
    let base_media_decode_time = scale.from_nanos(first_sample_dts).ok_or(
        Error::new(ErrorKind::Overflow).with_message("Base media decode time exceeds u64 range"),
    )?;

    let tfdt = TfdtBox::new(base_media_decode_time);
    let truns = build_truns(&track.sample_table, &track.defaults, &scale)?;

    Ok(TrafBox {
        tfhd,
        tfdt: Some(tfdt),
        truns,
        sbgps: Vec::new(),
        sgpds: Vec::new(),
    })
}

fn build_truns(
    sample_table: &SampleTable,
    track_defaults: &TrackDefaults,
    scale: &TickScale,
) -> Result<Vec<TrunBox>> {
    let mut truns = Vec::with_capacity(sample_table.chunks.len());

    let default_sample_duration = track_defaults
        .sample_duration
        .and_then(|d| scale.from_duration(d))
        .and_then(|ticks| u32::try_from(ticks).ok());
    let mut entries: Vec<TrunEntry> = Vec::new();

    for chunk in sample_table.chunks.iter() {
        entries.reserve(chunk.sample_count());

        let mut use_default_duration = true;
        let mut use_default_size = true;
        let mut rest_all_default_flags = true;
        let mut first_flags_matches_default = true;
        let mut has_nonzero_cto = false;

        for sample in chunk.samples() {
            let sample_duration = scale.sample_delta(sample)?;
            let sample_size = sample.size;
            let sample_flags = if sample.is_sync {
                SampleFlags::sync()
            } else {
                SampleFlags::non_sync()
            };
            let cto_ticks = scale.composition_time_offset(sample)?;
            has_nonzero_cto |= cto_ticks != 0;

            if Some(sample_duration) != default_sample_duration {
                use_default_duration = false;
            }

            if Some(sample_size) != track_defaults.sample_size {
                use_default_size = false;
            }

            if entries.is_empty() {
                first_flags_matches_default = Some(sample_flags) == track_defaults.sample_flags;
            } else if rest_all_default_flags && Some(sample_flags) != track_defaults.sample_flags {
                rest_all_default_flags = false;
            }

            entries.push(TrunEntry {
                duration: Some(sample_duration),
                size: Some(sample_size),
                flags: Some(sample_flags),
                composition_time_offset: Some(cto_ticks),
            });
        }

        let use_first_sample_flags = rest_all_default_flags && !first_flags_matches_default;
        let use_default_flags = rest_all_default_flags && first_flags_matches_default;

        let mut trun = TrunBox::default();
        trun.set_data_offset(i32::try_from(chunk.data_offset())?);

        if use_first_sample_flags {
            trun.set_first_sample_flags(entries[0].flags.unwrap());
        }

        for mut entry in entries.drain(..) {
            if use_default_duration {
                entry.duration = None;
            }
            if use_default_size {
                entry.size = None;
            }
            if use_default_flags || use_first_sample_flags {
                entry.flags = None;
            }
            if !has_nonzero_cto {
                // Ctts box must be used if any sample has a non-zero composition time offset
                entry.composition_time_offset = None;
            }

            trun.try_push_entry(entry).map_err(Error::box_encode)?;
        }

        truns.push(trun);
    }

    Ok(truns)
}

pub(super) fn build_mvex(movie: &Movie) -> Result<MvexBox> {
    let mut trexs = Vec::with_capacity(movie.tracks.len());
    for track in movie.tracks.iter() {
        let trex = build_trex(track)?;
        trexs.push(trex);
    }

    Ok(MvexBox { mehd: None, trexs })
}

fn build_trex(track: &Track) -> Result<TrexBox> {
    let scale = TickScale(track.timescale);
    let default_sample_duration = track
        .defaults
        .sample_duration
        .and_then(|d| scale.from_duration(d))
        .ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Default sample duration exceeds u64 range"),
        )?;

    let default_sample_duration = u32::try_from(default_sample_duration)?;

    Ok(TrexBox {
        track_id: track.id.get(),
        default_sample_description_index: 1,
        default_sample_duration: default_sample_duration,
        default_sample_size: track.defaults.sample_size.unwrap_or_default(),
        default_sample_flags: track.defaults.sample_flags.unwrap_or_default(),
        ..Default::default()
    })
}

/// Timescale-bound converter for nanosecond ↔ tick transformations.
///
/// All conversion methods share a single `from_nanos` base, which guarantees
/// the telescoping-sum property: the sum of per-sample deltas produced by
/// [`sample_delta`](Self::sample_delta) equals the total returned by
/// [`media_duration`](Self::media_duration).
struct TickScale(NonZeroU32);

impl TickScale {
    /// Converts nanoseconds to ticks at this timescale.
    fn from_nanos(&self, nanos: u64) -> Option<u64> {
        let ts = u64::from(self.0.get());
        let secs = nanos / 1_000_000_000;
        let sub_nanos = nanos % 1_000_000_000;

        secs.checked_mul(ts)?
            .checked_add(sub_nanos * ts / 1_000_000_000)
    }

    /// Converts a `Duration` to ticks at this timescale.
    fn from_duration(&self, duration: Duration) -> Option<u64> {
        let ts = u64::from(self.0.get());
        let secs = duration.as_secs();
        let sub_nanos = u64::from(duration.subsec_nanos());

        secs.checked_mul(ts)?
            .checked_add(sub_nanos * ts / 1_000_000_000)
    }

    /// Computes a sample's duration in ticks using the telescoping sum:
    /// `ticks(dts + dur) − ticks(dts)`.
    fn sample_delta(&self, sample: &Sample) -> Result<u32> {
        let dts_ticks = self
            .from_nanos(sample.dts_ns)
            .ok_or(Error::new(ErrorKind::Overflow).with_message("Sample DTS exceeds u64 range"))?;
        let end_ns = sample.dts_ns + sample.duration().as_nanos() as u64;
        let end_ticks = self.from_nanos(end_ns).ok_or(
            Error::new(ErrorKind::Overflow).with_message("Sample duration exceeds u64 range"),
        )?;
        u32::try_from(end_ticks - dts_ticks).map_err(|_| {
            Error::new(ErrorKind::Overflow).with_message("Sample duration exceeds u32 range")
        })
    }

    /// Computes the total media duration in ticks using the telescoping sum:
    /// `ticks(last_end) − ticks(first_start)`.
    fn media_duration(&self, sample_table: &SampleTable) -> Result<u64> {
        let last_sample = sample_table.chunks.last().map(|c| c.last());

        let Some(last) = last_sample else {
            return Ok(0);
        };

        let first_dts_ns = sample_table.first_sample_dts().unwrap_or(0);
        let end_dts_ns = last.dts_ns + last.duration().as_nanos() as u64;

        let first_ticks = self
            .from_nanos(first_dts_ns)
            .ok_or(Error::new(ErrorKind::Overflow).with_message("first DTS"))?;
        let end_ticks = self
            .from_nanos(end_dts_ns)
            .ok_or(Error::new(ErrorKind::Overflow).with_message("end DTS"))?;

        Ok(end_ticks - first_ticks)
    }

    /// Computes a track's effective duration in ticks.
    /// Uses edit list duration if present, otherwise media duration.
    fn track_duration(&self, track: &Track) -> Result<u64> {
        if let Some(edit_duration) = track.edit_duration() {
            self.from_duration(edit_duration)
                .ok_or(Error::new(ErrorKind::Overflow).with_message("edit list duration"))
        } else {
            self.media_duration(&track.sample_table)
        }
    }

    /// Converts a sample's composition time offset from nanoseconds to ticks.
    /// Returns 0 if the sample has no CTO or it is zero.
    fn composition_time_offset(&self, sample: &Sample) -> Result<i64> {
        let Some(cto) = sample.composition_time_offset_ns() else {
            return Ok(0);
        };
        if cto == 0 {
            return Ok(0);
        }
        let ticks = self.from_nanos(cto.unsigned_abs()).ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Composition time offset exceeds u64 range"),
        )?;
        let ticks = i64::try_from(ticks)?;
        Ok(if cto < 0 { -ticks } else { ticks })
    }
}

#[cfg(test)]
mod tests {
    use super::super::Sample;
    use super::super::types::Chunk;
    use super::*;

    fn nz(v: u32) -> NonZeroU32 {
        NonZeroU32::new(v).unwrap()
    }

    fn make_sample(
        dts_ns: u64,
        pts_ns: Option<i64>,
        duration_ms: u64,
        is_sync: bool,
        size: u32,
        data_offset: u64,
    ) -> Sample {
        Sample::new(
            dts_ns,
            pts_ns,
            Duration::from_millis(duration_ms),
            is_sync,
            size,
            data_offset,
        )
    }

    fn make_sample_table(chunks: Vec<Chunk>) -> SampleTable {
        SampleTable { chunks }
    }

    // --- TickScale::from_nanos ---

    #[test]
    fn from_nanos_zero() {
        assert_eq!(TickScale(nz(90000)).from_nanos(0), Some(0));
    }

    #[test]
    fn from_nanos_one_second() {
        // 1 second at 90kHz = 90000 ticks
        assert_eq!(TickScale(nz(90000)).from_nanos(1_000_000_000), Some(90000));
    }

    #[test]
    fn from_nanos_subsecond() {
        // 500ms at 1000Hz = 500 ticks
        assert_eq!(TickScale(nz(1000)).from_nanos(500_000_000), Some(500));
    }

    #[test]
    fn from_nanos_overflow() {
        // Large seconds * large timescale overflows u64
        let large_nanos = u64::MAX / 2;
        assert!(TickScale(nz(u32::MAX)).from_nanos(large_nanos).is_none());
    }

    // --- TickScale::from_duration ---

    #[test]
    fn from_duration_one_second() {
        let d = Duration::from_secs(1);
        assert_eq!(TickScale(nz(44100)).from_duration(d), Some(44100));
    }

    #[test]
    fn from_duration_fractional() {
        let d = Duration::from_millis(33);
        // 33ms at 90kHz = 2970 ticks
        assert_eq!(TickScale(nz(90000)).from_duration(d), Some(2970));
    }

    #[test]
    fn from_duration_zero() {
        assert_eq!(TickScale(nz(90000)).from_duration(Duration::ZERO), Some(0));
    }

    // --- build_truns ---

    #[test]
    fn build_truns_all_defaults() {
        // All samples match defaults → duration/size/flags all None in entries
        let defaults = TrackDefaults {
            sample_duration: Some(Duration::from_millis(33)),
            sample_size: Some(1000),
            sample_flags: Some(SampleFlags::sync()),
        };

        let mut chunk = Chunk::new(make_sample(0, None, 33, true, 1000, 0));
        chunk.try_push(make_sample(33_000_000, None, 33, true, 1000, 1000));

        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert_eq!(truns.len(), 1);
        let trun = &truns[0];
        assert_eq!(trun.entries().len(), 2);

        // All fields should be None (using defaults)
        for entry in trun.entries() {
            assert!(entry.duration.is_none());
            assert!(entry.size.is_none());
            assert!(entry.flags.is_none());
            assert!(entry.composition_time_offset.is_none());
        }
    }

    #[test]
    fn build_truns_no_defaults() {
        // No defaults → all fields present in entries
        let defaults = TrackDefaults::default();

        let mut chunk = Chunk::new(make_sample(0, None, 33, true, 1000, 0));
        chunk.try_push(make_sample(33_000_000, None, 33, true, 2000, 1000));

        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        let trun = &truns[0];
        for entry in trun.entries() {
            assert!(entry.duration.is_some());
            assert!(entry.size.is_some());
            assert!(entry.flags.is_some());
        }
    }

    #[test]
    fn build_truns_first_sample_flags() {
        // First sample is sync, rest are non-sync with default = non_sync
        // → first_sample_flags should be set, per-sample flags should be None
        let defaults = TrackDefaults {
            sample_duration: Some(Duration::from_millis(33)),
            sample_size: Some(1000),
            sample_flags: Some(SampleFlags::non_sync()),
        };

        let mut chunk = Chunk::new(make_sample(0, None, 33, true, 1000, 0));
        chunk.try_push(make_sample(33_000_000, None, 33, false, 1000, 1000));
        chunk.try_push(make_sample(66_000_000, None, 33, false, 1000, 2000));

        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        let trun = &truns[0];
        assert_eq!(trun.first_sample_flags(), Some(SampleFlags::sync()));
        for entry in trun.entries() {
            assert!(entry.flags.is_none());
        }
    }

    #[test]
    fn build_truns_per_sample_flags() {
        // Mixed sync patterns beyond first sample → per-sample flags needed
        let defaults = TrackDefaults {
            sample_duration: Some(Duration::from_millis(33)),
            sample_size: Some(1000),
            sample_flags: Some(SampleFlags::non_sync()),
        };

        let mut chunk = Chunk::new(make_sample(0, None, 33, true, 1000, 0));
        chunk.try_push(make_sample(33_000_000, None, 33, false, 1000, 1000));
        chunk.try_push(make_sample(66_000_000, None, 33, true, 1000, 2000)); // sync in the middle

        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        let trun = &truns[0];
        assert!(trun.first_sample_flags().is_none());
        for entry in trun.entries() {
            assert!(entry.flags.is_some());
        }
    }

    #[test]
    fn build_truns_with_cto() {
        // Samples with non-zero composition time offset
        let defaults = TrackDefaults::default();

        // dts=0, pts=66ms → CTO=66ms
        let chunk = Chunk::new(make_sample(0, Some(66_000_000), 33, true, 1000, 0));
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        let trun = &truns[0];
        let entry = &trun.entries()[0];
        assert!(entry.composition_time_offset.is_some());
        // 66ms at 90kHz = 5940 ticks
        assert_eq!(entry.composition_time_offset, Some(5940));
    }

    #[test]
    fn build_truns_zero_cto_omitted() {
        // All CTOs are zero → composition_time_offset should be None
        let defaults = TrackDefaults::default();

        let chunk = Chunk::new(make_sample(0, Some(0), 33, true, 1000, 0));
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        let trun = &truns[0];
        let entry = &trun.entries()[0];
        assert!(entry.composition_time_offset.is_none());
    }

    #[test]
    fn build_truns_multiple_chunks() {
        let defaults = TrackDefaults::default();

        let chunk1 = Chunk::new(make_sample(0, None, 33, true, 1000, 0));
        let chunk2 = Chunk::new(make_sample(33_000_000, None, 33, true, 2000, 5000));

        let table = make_sample_table(vec![chunk1, chunk2]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert_eq!(truns.len(), 2);
        assert_eq!(truns[0].data_offset(), Some(0));
        assert_eq!(truns[1].data_offset(), Some(5000));
    }

    #[test]
    fn build_truns_data_offset_set() {
        let defaults = TrackDefaults::default();
        let chunk = Chunk::new(make_sample(0, None, 33, true, 1000, 256));
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert_eq!(truns[0].data_offset(), Some(256));
    }

    #[test]
    fn build_truns_duration_ticks_conversion() {
        let defaults = TrackDefaults::default();

        // 33ms at 90kHz = 2970 ticks
        let chunk = Chunk::new(make_sample(0, None, 33, true, 1000, 0));
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        let entry = &truns[0].entries()[0];
        assert_eq!(entry.duration, Some(2970));
    }

    // --- telescoping sum roundtrip ---

    /// Helper: converts ticks to nanos without accumulated rounding error.
    /// Uses the same formula as nanos_to_ticks but in reverse.
    fn ticks_to_nanos(ticks: u64, timescale: u64) -> u64 {
        let secs = ticks / timescale;
        let remainder = ticks % timescale;
        secs * 1_000_000_000 + remainder * 1_000_000_000 / timescale
    }

    /// Verifies the telescoping sum property: the sum of per-sample delta ticks
    /// equals from_nanos(end) - from_nanos(start), i.e. no rounding
    /// error accumulates across samples.
    #[test]
    fn telescoping_sum_consistency_uniform_delta() {
        // 512 ticks/sample at 15360 Hz (30fps video, typical H.264)
        // 512/15360 = 1/30 sec = 33333333.333... ns — not exactly representable
        let timescale: u64 = 15360;
        let delta_ticks: u64 = 512;
        let num_samples: u64 = 10;
        let scale = TickScale(nz(timescale as u32));

        let mut samples = Vec::new();
        for i in 0..num_samples {
            let dts_ticks = i * delta_ticks;
            let dts_ns = ticks_to_nanos(dts_ticks, timescale);
            let next_dts_ns = ticks_to_nanos(dts_ticks + delta_ticks, timescale);
            let duration_ns = next_dts_ns - dts_ns;
            samples.push(Sample::new(
                dts_ns,
                None,
                Duration::from_nanos(duration_ns),
                true,
                100,
                0,
            ));
        }

        let mut total_delta: u64 = 0;
        for sample in &samples {
            total_delta += u64::from(scale.sample_delta(sample).unwrap());
        }

        // Telescoping sum: sum of deltas == from_nanos(end) - from_nanos(start)
        let start_ns = samples.first().unwrap().dts_ns;
        let last = samples.last().unwrap();
        let end_ns = last.dts_ns + last.duration().as_nanos() as u64;
        let expected = scale.from_nanos(end_ns).unwrap() - scale.from_nanos(start_ns).unwrap();

        assert_eq!(total_delta, expected);
    }

    #[test]
    fn telescoping_sum_consistency_variable_delta() {
        // Variable frame durations at 48000 Hz (audio timescale)
        let timescale: u64 = 48000;
        let deltas: &[u64] = &[1024, 1024, 960, 1024, 1024, 960, 1024];
        let scale = TickScale(nz(timescale as u32));

        let mut dts_ticks: u64 = 0;
        let mut samples = Vec::new();
        for &dt in deltas {
            let dts_ns = ticks_to_nanos(dts_ticks, timescale);
            let next_dts_ns = ticks_to_nanos(dts_ticks + dt, timescale);
            let duration_ns = next_dts_ns - dts_ns;
            samples.push(Sample::new(
                dts_ns,
                None,
                Duration::from_nanos(duration_ns),
                true,
                100,
                0,
            ));
            dts_ticks += dt;
        }

        let mut total_delta: u64 = 0;
        for sample in &samples {
            total_delta += u64::from(scale.sample_delta(sample).unwrap());
        }

        let start_ns = samples.first().unwrap().dts_ns;
        let last = samples.last().unwrap();
        let end_ns = last.dts_ns + last.duration().as_nanos() as u64;
        let expected = scale.from_nanos(end_ns).unwrap() - scale.from_nanos(start_ns).unwrap();

        assert_eq!(total_delta, expected);
    }

    #[test]
    fn telescoping_sum_consistency_media_duration() {
        // Verify media_duration equals the sum of sample_delta
        let timescale: u64 = 15360;
        let delta_ticks: u64 = 512;
        let num_samples: u64 = 100;
        let scale = TickScale(nz(timescale as u32));

        let mut chunk = {
            let dts_ns = ticks_to_nanos(0, timescale);
            let next_dts_ns = ticks_to_nanos(delta_ticks, timescale);
            Chunk::new(Sample::new(
                dts_ns,
                None,
                Duration::from_nanos(next_dts_ns - dts_ns),
                true,
                100,
                0,
            ))
        };

        for i in 1..num_samples {
            let dts_ticks = i * delta_ticks;
            let dts_ns = ticks_to_nanos(dts_ticks, timescale);
            let next_dts_ns = ticks_to_nanos(dts_ticks + delta_ticks, timescale);
            let duration_ns = next_dts_ns - dts_ns;
            chunk.try_push(Sample::new(
                dts_ns,
                None,
                Duration::from_nanos(duration_ns),
                true,
                100,
                i * 100,
            ));
        }

        let table = make_sample_table(vec![chunk]);
        let duration = scale.media_duration(&table).unwrap();

        // Sum of per-sample deltas must equal media_duration
        let mut sum_deltas: u64 = 0;
        for sample in table.chunks.iter().flat_map(|c| c.samples()) {
            sum_deltas += u64::from(scale.sample_delta(sample).unwrap());
        }

        assert_eq!(duration, sum_deltas);
    }

    #[test]
    fn telescoping_sum_exact_when_divisible() {
        // When timescale divides 1e9, the roundtrip is lossless
        // 1000 Hz: 1 tick = 1_000_000 ns (exact)
        let timescale: u64 = 1000;
        let delta_ticks: u64 = 33;
        let num_samples: u64 = 100;
        let scale = TickScale(nz(timescale as u32));

        let mut samples = Vec::new();
        for i in 0..num_samples {
            let dts_ticks = i * delta_ticks;
            let dts_ns = ticks_to_nanos(dts_ticks, timescale);
            let next_dts_ns = ticks_to_nanos(dts_ticks + delta_ticks, timescale);
            let duration_ns = next_dts_ns - dts_ns;
            samples.push(Sample::new(
                dts_ns,
                None,
                Duration::from_nanos(duration_ns),
                true,
                100,
                0,
            ));
        }

        // When divisible, each individual delta should recover exactly
        for sample in &samples {
            let dt = scale.sample_delta(sample).unwrap();
            assert_eq!(u64::from(dt), delta_ticks);
        }
    }

    #[test]
    fn telescoping_sum_cross_timescale() {
        // Media at 15360 Hz, movie at 1000 Hz
        // Total = 512 * 3 = 1536 ticks at 15360 Hz = 0.1 sec = 100 ticks at 1000 Hz
        let media_ts: u64 = 15360;
        let movie_ts: u64 = 1000;
        let delta_ticks: u64 = 512;
        let num_samples: u64 = 3;

        let mut chunk = {
            let dts_ns = ticks_to_nanos(0, media_ts);
            let next_dts_ns = ticks_to_nanos(delta_ticks, media_ts);
            Chunk::new(Sample::new(
                dts_ns,
                None,
                Duration::from_nanos(next_dts_ns - dts_ns),
                true,
                100,
                0,
            ))
        };

        for i in 1..num_samples {
            let dts_ticks = i * delta_ticks;
            let dts_ns = ticks_to_nanos(dts_ticks, media_ts);
            let next_dts_ns = ticks_to_nanos(dts_ticks + delta_ticks, media_ts);
            chunk.try_push(Sample::new(
                dts_ns,
                None,
                Duration::from_nanos(next_dts_ns - dts_ns),
                true,
                100,
                i * 100,
            ));
        }

        let table = make_sample_table(vec![chunk]);

        // Cross-timescale: 0.1 sec at 1000 Hz = exactly 100 ticks
        let movie_scale = TickScale(nz(movie_ts as u32));
        let movie_dur = movie_scale.media_duration(&table).unwrap();
        assert_eq!(movie_dur, 100);
    }
}
