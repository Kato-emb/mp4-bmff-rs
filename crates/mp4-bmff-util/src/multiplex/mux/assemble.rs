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
    VisualSampleDescription,
};

use super::types::{
    Movie, //
    SampleTable,
    Track,
    TrackDefaults,
};

// ---------------------------------------------------------------------------
// TickScale — timescale-bound converter
// ---------------------------------------------------------------------------

/// Timescale-bound converter for nanosecond ↔ tick transformations.
///
/// All conversion methods share a single [`from_nanos`](Self::from_nanos)
/// base, which guarantees the telescoping-sum property: the sum of
/// per-sample deltas produced by [`delta`](Self::delta) equals the total
/// span computed from the first and last timestamps.
///
/// This type only accepts primitive / std types (`u64`, `i64`, `Duration`) as
/// input, keeping it independent of domain types like `Sample` or `Track`.
struct TickScale(NonZeroU32);

impl TickScale {
    /// Converts nanoseconds to ticks.
    ///
    /// The sub-second term `sub_nanos * ts / 1_000_000_000` uses integer
    /// division and therefore truncates. This truncation is intentional:
    /// because [`delta`](Self::delta) computes `from_nanos(end) − from_nanos(start)`,
    /// the rounding errors cancel out (telescoping sum), so the sum of
    /// per-sample deltas always equals the total span exactly.
    fn from_nanos(&self, nanos: u64) -> Option<u64> {
        let ts = u64::from(self.0.get());
        let secs = nanos / 1_000_000_000;
        let sub_nanos = nanos % 1_000_000_000;

        secs.checked_mul(ts)?
            .checked_add(sub_nanos * ts / 1_000_000_000)
    }

    /// Converts a signed nanosecond value to signed ticks.
    fn signed_from_nanos(&self, nanos: i64) -> Option<i64> {
        let ticks = i64::try_from(self.from_nanos(nanos.unsigned_abs())?).ok()?;
        Some(if nanos < 0 { -ticks } else { ticks })
    }

    /// Converts a `Duration` to ticks.
    fn from_duration(&self, duration: Duration) -> Option<u64> {
        let ts = u64::from(self.0.get());
        let secs = duration.as_secs();
        let sub_nanos = u64::from(duration.subsec_nanos());

        secs.checked_mul(ts)?
            .checked_add(sub_nanos * ts / 1_000_000_000)
    }

    /// Computes the tick difference between two nanosecond timestamps:
    /// `ticks(end_ns) − ticks(start_ns)`.
    fn delta(&self, start_ns: u64, end_ns: u64) -> Option<u64> {
        self.from_nanos(end_ns)?
            .checked_sub(self.from_nanos(start_ns)?)
    }
}

// ---------------------------------------------------------------------------
// Domain helpers — bridge between domain types and TickScale
// ---------------------------------------------------------------------------

/// Computes the total media duration in ticks via the telescoping sum.
fn media_duration(table: &SampleTable, scale: &TickScale) -> Result<u64> {
    let Some(last) = table.chunks.last().and_then(|c| c.samples.last()) else {
        return Ok(0);
    };
    let start_ns = table.first_sample_dts().unwrap_or(0);
    let end_ns = u64::try_from(last.duration.as_nanos())
        .ok()
        .and_then(|d| last.dts_ns.checked_add(d))
        .ok_or(Error::new(ErrorKind::Overflow).with_message("Media duration exceeds u64 range"))?;

    scale.delta(start_ns, end_ns).ok_or(
        Error::new(ErrorKind::Overflow).with_message("Media duration in ticks exceeds u64 range"),
    )
}

/// Computes a track's effective duration in ticks.
/// Uses edit list duration if present, otherwise media duration.
fn track_duration(track: &Track, scale: &TickScale) -> Result<u64> {
    if let Some(edit_dur) = track.edit_duration() {
        scale
            .from_duration(edit_dur)
            .ok_or(Error::new(ErrorKind::Overflow).with_message("Edit list duration overflow"))
    } else {
        media_duration(&track.sample_table, scale)
    }
}

/// Computes a sample's duration in `u32` ticks via the telescoping sum.
fn sample_delta_ticks(dts_ns: u64, duration: Duration, scale: &TickScale) -> Result<u32> {
    let start_ns = dts_ns;
    let end_ns = u64::try_from(duration.as_nanos())
        .ok()
        .and_then(|d| dts_ns.checked_add(d))
        .ok_or(Error::new(ErrorKind::Overflow).with_message("Sample end time exceeds u64 range"))?;

    scale
        .delta(start_ns, end_ns)
        .and_then(|d| u32::try_from(d).ok())
        .ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Sample duration in ticks exceeds u32 range"),
        )
}

/// Converts a composition time offset (nanoseconds) to ticks.
fn sample_cto_ticks(dts_ns: u64, pts_ns: Option<i64>, scale: &TickScale) -> Result<i64> {
    let Some(pts_ns) = pts_ns else {
        return Ok(0);
    };

    let cto_ns = if pts_ns >= 0 {
        let pts_uns = pts_ns as u64;
        if pts_uns >= dts_ns {
            i64::try_from(pts_uns - dts_ns).ok()
        } else {
            i64::try_from(dts_ns - pts_uns).ok().map(|v| -v)
        }
    } else {
        let abs_pts_ns = pts_ns.unsigned_abs();
        dts_ns
            .checked_add(abs_pts_ns)
            .and_then(|v| i64::try_from(v).ok())
            .map(|v| -v)
    }
    .ok_or_else(|| {
        Error::new(ErrorKind::Overflow)
            .with_message("Composition time offset (PTS - DTS) exceeds i64 range")
    })?;

    scale.signed_from_nanos(cto_ns).ok_or_else(|| {
        Error::new(ErrorKind::Overflow)
            .with_message("Composition time offset in ticks exceeds i64 range")
    })
}

/// Converts an optional default sample duration to `u32` ticks.
///
/// Returns an error if the duration is set but overflows the tick range,
/// rather than silently discarding it.
fn default_duration_ticks(duration: Option<Duration>, scale: &TickScale) -> Result<Option<u32>> {
    duration
        .map(|d| {
            let ticks = scale.from_duration(d).ok_or(
                Error::new(ErrorKind::Overflow)
                    .with_message("Default sample duration exceeds tick range"),
            )?;
            u32::try_from(ticks).map_err(|_| {
                Error::new(ErrorKind::Overflow)
                    .with_message("Default sample duration exceeds u32 range")
            })
        })
        .transpose()
}

// ---------------------------------------------------------------------------
// Non-fragmented: moov
// ---------------------------------------------------------------------------

pub(super) fn build_moov(movie: &Movie) -> Result<MoovBox> {
    let mvhd = build_mvhd(movie)?;
    let traks = movie
        .tracks
        .iter()
        .map(|t| build_trak(t, movie.timescale))
        .collect::<Result<Vec<_>>>()?;

    Ok(MoovBox {
        mvhd,
        traks,
        mvex: None,
    })
}

fn build_mvhd(movie: &Movie) -> Result<MvhdBox> {
    let scale = TickScale(movie.timescale);
    let duration = movie
        .tracks
        .iter()
        .map(|t| track_duration(t, &scale))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(0);

    Ok(MvhdBox {
        timescale: movie.timescale.get(),
        duration,
        next_track_id: movie.next_track_id()?.get(),
        ..Default::default()
    })
}

fn build_trak(track: &Track, movie_timescale: NonZeroU32) -> Result<TrakBox> {
    Ok(TrakBox {
        tkhd: build_tkhd(track, movie_timescale)?,
        mdia: build_mdia(track)?,
        edts: build_edts(track, movie_timescale)?,
        tref: None,
        trgr: None,
    })
}

fn build_tkhd(track: &Track, movie_timescale: NonZeroU32) -> Result<TkhdBox> {
    let duration = track_duration(track, &TickScale(movie_timescale))?;
    let mut tkhd = TkhdBox::new(track.id.into_inner(), duration);
    tkhd.alternate_group = track.alternate_group;
    tkhd.matrix = track.matrix;

    match &track.media {
        MediaDefinition::Video { width, height, .. } => {
            tkhd.width = U16F16::from_integer(i128::from(*width));
            tkhd.height = U16F16::from_integer(i128::from(*height));
        }
        MediaDefinition::Audio { .. } => {
            tkhd.volume = U8F8::from_f32(1.0);
        }
        _ => {} // Other media types use default tkhd values
    }

    Ok(tkhd)
}

fn build_mdia(track: &Track) -> Result<MdiaBox> {
    Ok(MdiaBox {
        mdhd: build_mdhd(track)?,
        hdlr: HdlrBox::new(track.media.handler_type()),
        minf: build_minf(track)?,
        elng: None,
    })
}

fn build_mdhd(track: &Track) -> Result<MdhdBox> {
    let duration = media_duration(&track.sample_table, &TickScale(track.timescale))?;
    Ok(MdhdBox::new(track.timescale, duration, track.language))
}

fn build_minf(track: &Track) -> Result<MinfBox> {
    let scale = TickScale(track.timescale);
    Ok(MinfBox {
        media_header: build_media_header(track),
        stbl: build_stbl(&track.sample_table, build_stsd(track)?, &scale)?,
        dinf: DinfBox::self_contained(),
    })
}

fn build_media_header(track: &Track) -> MediaHeaderBox {
    match &track.media {
        MediaDefinition::Video { .. } => MediaHeaderBox::Vmhd(VmhdBox::default()),
        MediaDefinition::Audio { .. } => MediaHeaderBox::Smhd(SmhdBox::default()),
        MediaDefinition::Hint { .. } => MediaHeaderBox::Hmhd(HmhdBox::default()),
        _ => MediaHeaderBox::Nmhd(NmhdBox::default()),
    }
}

fn build_stsd(track: &Track) -> Result<StsdBox> {
    let entry = build_sample_entry(&track.media)?;

    Ok(StsdBox {
        entries: alloc::vec![entry],
        ..Default::default()
    })
}

fn build_sample_entry(media: &MediaDefinition) -> Result<RawBoxOwned> {
    match media {
        MediaDefinition::Video {
            width,
            height,
            codec,
        } => build_visual_sample_entry(*width, *height, codec),
        MediaDefinition::Audio {
            channel_count,
            sample_rate,
            codec,
        } => build_audio_sample_entry(*channel_count, *sample_rate, codec),
        _ => Err(Error::new(ErrorKind::Unsupported)
            .with_message("Unsupported media definition for sample entry construction")),
    }
}

fn build_visual_sample_entry(
    width: u16,
    height: u16,
    codec: &VisualSampleDescription,
) -> Result<RawBoxOwned> {
    let base = VisualSampleEntry {
        width,
        height,
        ..Default::default()
    };

    match codec {
        VisualSampleDescription::Avc1(avc_config) => {
            use mp4_bmff::boxes::avc::Avc1SampleEntry;
            use mp4_bmff::boxes::avc::AvcCBox;
            let avcc = AvcCBox {
                avc_config: avc_config.clone(),
            };

            let avc1 = Avc1SampleEntry::new(base, avcc, None, None);
            RawBoxOwned::from_boxed(&avc1).map_err(|e| Error::box_encode(e))
        }
        VisualSampleDescription::Avc3(avc_config) => {
            use mp4_bmff::boxes::avc::Avc3SampleEntry;
            use mp4_bmff::boxes::avc::AvcCBox;
            let avcc = AvcCBox {
                avc_config: avc_config.clone(),
            };

            let avc3 = Avc3SampleEntry::new(base, avcc, None, None);
            RawBoxOwned::from_boxed(&avc3).map_err(|e| Error::box_encode(e))
        }
        VisualSampleDescription::Mp4v(dec_specific_info) => {
            use mp4_bmff::formats::mpeg4::systems::descriptor::DecoderConfigDescriptor;
            use mp4_bmff::formats::mpeg4::systems::descriptor::EsDescriptor;
            let dec_config = DecoderConfigDescriptor::mp4v(dec_specific_info);
            // TODO: MP4仕様（ISO 14496-14）では、esds box内の es_id は track_id と一致させるか、0 にすることが推奨されている
            let esd = EsDescriptor::new(0, dec_config);

            use mp4_bmff::boxes::mp4::EsdsBox;
            use mp4_bmff::boxes::mp4::Mp4vSampleEntry;
            let esds = EsdsBox::new(esd);
            let mp4v = Mp4vSampleEntry { base, esds };
            RawBoxOwned::from_boxed(&mp4v).map_err(|e| Error::box_encode(e))
        }
    }
}

fn build_audio_sample_entry(
    channel_count: u16,
    sample_rate: u16,
    codec: &AudioSampleDescription,
) -> Result<RawBoxOwned> {
    let base = AudioSampleEntry {
        channelcount: channel_count,
        samplerate: U16F16::from_integer(i128::from(sample_rate)),
        ..Default::default()
    };

    match codec {
        AudioSampleDescription::Mp4a(dec_specific_info) => {
            use mp4_bmff::formats::mpeg4::systems::descriptor::DecoderConfigDescriptor;
            use mp4_bmff::formats::mpeg4::systems::descriptor::EsDescriptor;
            let dec_config = DecoderConfigDescriptor::mp4a(dec_specific_info);

            let esd = EsDescriptor::new(0, dec_config);

            use mp4_bmff::boxes::mp4::EsdsBox;
            use mp4_bmff::boxes::mp4::Mp4aSampleEntry;
            let esds = EsdsBox::new(esd);
            let mp4a = Mp4aSampleEntry { base, esds };
            RawBoxOwned::from_boxed(&mp4a).map_err(|e| Error::box_encode(e))
        }
    }
}

fn build_stbl(table: &SampleTable, stsd: StsdBox, scale: &TickScale) -> Result<StblBox> {
    let mut stts = SttsBox::default();
    let mut stsz = StszBox::default();
    let mut stsc = StscBox::default();
    let mut ctts = CttsBox::default();
    let mut stss = StssBox::default();
    let mut chunk_offset = ChunkOffset::default();

    let mut has_nonzero_ctts = false;
    let mut chunk_number: u32 = 0;
    let mut sample_number: u32 = 0;

    for chunk in table.chunks.iter() {
        chunk_number = chunk_number.checked_add(1).ok_or(ErrorKind::Overflow)?;
        let samples_per_chunk = u32::try_from(chunk.samples.len()).map_err(|_| {
            Error::new(ErrorKind::Overflow)
                .with_message("Number of samples per chunk exceeds u32 range")
        })?;
        stsc.push(chunk_number, samples_per_chunk, 1);
        chunk_offset.push(chunk.data_offset);

        for sample in &chunk.samples {
            sample_number = sample_number.checked_add(1).ok_or(ErrorKind::Overflow)?;

            let delta = sample_delta_ticks(sample.dts_ns, sample.duration, scale)?;
            let cto = sample_cto_ticks(sample.dts_ns, sample.pts_ns, scale).and_then(|cto| {
                i32::try_from(cto).map_err(|_| {
                    Error::new(ErrorKind::Overflow)
                        .with_message("Composition time offset exceeds i32 tick range")
                })
            })?;

            stts.push(delta);
            stsz.push(sample.size);
            ctts.push(cto);
            has_nonzero_ctts |= cto != 0;

            if sample.is_sync {
                stss.push(sample_number);
            }
        }
    }

    // Omit ctts if all offsets are zero.
    let ctts = if has_nonzero_ctts { Some(ctts) } else { None };
    // Omit stss if every sample is a sync sample (ISO 14496-12 §8.6.2).
    let stss = if stss.entries.len() as u32 == sample_number {
        None
    } else {
        Some(stss)
    };

    Ok(StblBox {
        stsd,
        stts,
        sample_size: SampleSize::Stsz(stsz),
        stsc,
        chunk_offset,
        ctts,
        stss,
        stdp: None,
        cslg: None,
        stsh: None,
        sdtp: None,
        sbgps: Vec::new(),
        sgpds: Vec::new(),
    })
}

fn build_edts(track: &Track, movie_timescale: NonZeroU32) -> Result<Option<EdtsBox>> {
    build_elst(track, movie_timescale).map(|elst| elst.map(|e| EdtsBox { elst: Some(e) }))
}

fn build_elst(track: &Track, movie_timescale: NonZeroU32) -> Result<Option<ElstBox>> {
    let Some(segments) = &track.edit_list else {
        return Ok(None);
    };

    let movie_scale = TickScale(movie_timescale);
    let media_scale = TickScale(track.timescale);

    let mut entries = Vec::with_capacity(segments.len());

    for seg in segments {
        let entry = match seg {
            EditSegment::Empty { duration } => ElstEntry {
                segment_duration: movie_scale
                    .from_duration(*duration)
                    .ok_or(ErrorKind::Overflow)?,
                media_time: -1,
                media_rate: I16F16::from_f32(1.0),
            },
            EditSegment::Media {
                duration,
                media_start,
                media_rate,
            } => ElstEntry {
                segment_duration: movie_scale
                    .from_duration(*duration)
                    .ok_or(ErrorKind::Overflow)?,
                media_time: media_scale
                    .from_duration(*media_start)
                    .and_then(|t| i64::try_from(t).ok())
                    .ok_or(
                        Error::new(ErrorKind::Overflow)
                            .with_message("Edit list media_start exceeds i64 tick range"),
                    )?,
                media_rate: I16F16::from_f32(*media_rate),
            },
            EditSegment::Dwell {
                duration,
                media_time,
            } => ElstEntry {
                segment_duration: movie_scale
                    .from_duration(*duration)
                    .ok_or(ErrorKind::Overflow)?,
                media_time: media_scale
                    .from_duration(*media_time)
                    .and_then(|t| i64::try_from(t).ok())
                    .ok_or(
                        Error::new(ErrorKind::Overflow)
                            .with_message("Edit list media_time exceeds i64 tick range"),
                    )?,
                media_rate: I16F16::from_f32(0.0),
            },
        };
        entries.push(entry);
    }

    Ok(Some(ElstBox::new(entries)))
}

// ---------------------------------------------------------------------------
// Fragmented: moof
// ---------------------------------------------------------------------------

pub(super) fn build_moof(movie: &Movie, sequence_number: u32) -> Result<MoofBox> {
    let trafs = movie
        .tracks
        .iter()
        .filter(|t| !t.sample_table.chunks.is_empty())
        .map(build_traf)
        .collect::<Result<Vec<_>>>()?;

    Ok(MoofBox {
        mfhd: MfhdBox::new(sequence_number),
        trafs,
    })
}

fn build_traf(track: &Track) -> Result<TrafBox> {
    let scale = TickScale(track.timescale);

    // --- tfhd ---
    let default_duration = default_duration_ticks(track.defaults.sample_duration, &scale)?;

    let mut flags = TfhdFlags::DEFAULT_BASE_IS_MOOF;
    if default_duration.is_some() {
        flags |= TfhdFlags::DEFAULT_SAMPLE_DURATION_PRESENT;
    }
    if track.defaults.sample_size.is_some() {
        flags |= TfhdFlags::DEFAULT_SAMPLE_SIZE_PRESENT;
    }
    if track.defaults.sample_flags.is_some() {
        flags |= TfhdFlags::DEFAULT_SAMPLE_FLAGS_PRESENT;
    }

    let tfhd = TfhdBox {
        flags,
        track_id: track.id.get(),
        default_sample_duration: default_duration,
        default_sample_size: track.defaults.sample_size,
        default_sample_flags: track.defaults.sample_flags,
        ..Default::default()
    };

    // --- tfdt ---
    let first_dts = track.sample_table.first_sample_dts().ok_or(
        Error::new(ErrorKind::InvalidInput)
            .with_message("Track must have at least one sample to build a traf box"),
    )?;
    let tfdt = TfdtBox::new(scale.from_nanos(first_dts).ok_or(
        Error::new(ErrorKind::Overflow).with_message("Base media decode time exceeds u64 range"),
    )?);

    // --- truns ---
    let truns = build_truns(&track.sample_table, &track.defaults, &scale)?;

    Ok(TrafBox {
        tfhd,
        tfdt: Some(tfdt),
        truns,
        sbgps: Vec::new(),
        sgpds: Vec::new(),
    })
}

/// Builds one `TrunBox` per chunk, omitting fields that match track defaults.
fn build_truns(
    table: &SampleTable,
    defaults: &TrackDefaults,
    scale: &TickScale,
) -> Result<Vec<TrunBox>> {
    let mut truns = Vec::with_capacity(table.chunks.len());

    let default_duration = default_duration_ticks(defaults.sample_duration, scale)?;

    let mut entries: Vec<TrunEntry> = Vec::new();

    for chunk in &table.chunks {
        entries.reserve(chunk.samples.len());

        // Phase 1: collect entries and track which fields can be omitted
        let mut all_duration_default = true;
        let mut all_size_default = true;
        let mut first_flags_matches = true;
        let mut rest_flags_default = true;
        let mut has_nonzero_cto = false;

        for (i, sample) in chunk.samples.iter().enumerate() {
            let duration = sample_delta_ticks(sample.dts_ns, sample.duration, scale)?;
            let size = sample.size;
            let flags = if sample.is_sync {
                SampleFlags::sync()
            } else {
                SampleFlags::non_sync()
            };
            let cto = sample_cto_ticks(sample.dts_ns, sample.pts_ns, scale)?;
            has_nonzero_cto |= cto != 0;

            if Some(duration) != default_duration {
                all_duration_default = false;
            }
            if Some(size) != defaults.sample_size {
                all_size_default = false;
            }
            if i == 0 {
                first_flags_matches = Some(flags) == defaults.sample_flags;
            } else if Some(flags) != defaults.sample_flags {
                rest_flags_default = false;
            }

            entries.push(TrunEntry {
                duration: Some(duration),
                size: Some(size),
                flags: Some(flags),
                composition_time_offset: Some(cto),
            });
        }

        // Phase 2: determine flags optimization strategy
        //
        // Three cases for sample_flags:
        //   1. All match default          → omit per-sample flags entirely
        //   2. Only first differs          → use first_sample_flags field
        //   3. Mixed across rest           → keep per-sample flags
        let omit_flags = rest_flags_default && first_flags_matches;
        let use_first_sample_flags = rest_flags_default && !first_flags_matches;

        let mut trun = TrunBox::default();
        trun.set_data_offset(i32::try_from(chunk.data_offset).map_err(|_| {
            Error::new(ErrorKind::Overflow).with_message("data_offset exceeds i32 range")
        })?);

        if use_first_sample_flags {
            trun.set_first_sample_flags(entries[0].flags.unwrap());
        }

        for mut entry in entries.drain(..) {
            if all_duration_default {
                entry.duration = None;
            }
            if all_size_default {
                entry.size = None;
            }
            if omit_flags || use_first_sample_flags {
                entry.flags = None;
            }
            if !has_nonzero_cto {
                entry.composition_time_offset = None;
            }
            trun.try_push_entry(entry).map_err(Error::box_encode)?;
        }

        truns.push(trun);
    }

    Ok(truns)
}

// ---------------------------------------------------------------------------
// Fragmented: mvex (movie extends header)
// ---------------------------------------------------------------------------

pub(super) fn build_mvex(movie: &Movie) -> Result<MvexBox> {
    let trexs = movie
        .tracks
        .iter()
        .map(build_trex)
        .collect::<Result<Vec<_>>>()?;

    Ok(MvexBox { mehd: None, trexs })
}

fn build_trex(track: &Track) -> Result<TrexBox> {
    let scale = TickScale(track.timescale);
    let default_sample_duration =
        default_duration_ticks(track.defaults.sample_duration, &scale)?.unwrap_or(0);

    Ok(TrexBox {
        track_id: track.id.get(),
        default_sample_description_index: 1,
        default_sample_duration,
        default_sample_size: track.defaults.sample_size.unwrap_or(0),
        default_sample_flags: track.defaults.sample_flags.unwrap_or_default(),
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    ) -> Sample {
        Sample::new(
            dts_ns,
            pts_ns,
            Duration::from_millis(duration_ms),
            is_sync,
            size,
        )
    }

    fn make_chunk(data_offset: u64, samples: Vec<Sample>) -> Chunk {
        Chunk {
            data_offset,
            samples,
        }
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
        assert_eq!(TickScale(nz(90000)).from_nanos(1_000_000_000), Some(90000));
    }

    #[test]
    fn from_nanos_subsecond() {
        assert_eq!(TickScale(nz(1000)).from_nanos(500_000_000), Some(500));
    }

    #[test]
    fn from_nanos_overflow() {
        let large_nanos = u64::MAX / 2;
        assert!(TickScale(nz(u32::MAX)).from_nanos(large_nanos).is_none());
    }

    // --- TickScale::from_duration ---

    #[test]
    fn from_duration_one_second() {
        assert_eq!(
            TickScale(nz(44100)).from_duration(Duration::from_secs(1)),
            Some(44100)
        );
    }

    #[test]
    fn from_duration_fractional() {
        assert_eq!(
            TickScale(nz(90000)).from_duration(Duration::from_millis(33)),
            Some(2970)
        );
    }

    #[test]
    fn from_duration_zero() {
        assert_eq!(TickScale(nz(90000)).from_duration(Duration::ZERO), Some(0));
    }

    // --- build_truns ---

    #[test]
    fn build_truns_all_defaults() {
        let defaults = TrackDefaults {
            sample_duration: Some(Duration::from_millis(33)),
            sample_size: Some(1000),
            sample_flags: Some(SampleFlags::sync()),
        };

        let chunk = make_chunk(
            0,
            vec![
                make_sample(0, None, 33, true, 1000),
                make_sample(33_000_000, None, 33, true, 1000),
            ],
        );

        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert_eq!(truns.len(), 1);
        let trun = &truns[0];
        assert_eq!(trun.entries().len(), 2);

        for entry in trun.entries() {
            assert!(entry.duration.is_none());
            assert!(entry.size.is_none());
            assert!(entry.flags.is_none());
            assert!(entry.composition_time_offset.is_none());
        }
    }

    #[test]
    fn build_truns_no_defaults() {
        let defaults = TrackDefaults::default();

        let chunk = make_chunk(
            0,
            vec![
                make_sample(0, None, 33, true, 1000),
                make_sample(33_000_000, None, 33, true, 2000),
            ],
        );

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
        let defaults = TrackDefaults {
            sample_duration: Some(Duration::from_millis(33)),
            sample_size: Some(1000),
            sample_flags: Some(SampleFlags::non_sync()),
        };

        let chunk = make_chunk(
            0,
            vec![
                make_sample(0, None, 33, true, 1000),
                make_sample(33_000_000, None, 33, false, 1000),
                make_sample(66_000_000, None, 33, false, 1000),
            ],
        );

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
        let defaults = TrackDefaults {
            sample_duration: Some(Duration::from_millis(33)),
            sample_size: Some(1000),
            sample_flags: Some(SampleFlags::non_sync()),
        };

        let chunk = make_chunk(
            0,
            vec![
                make_sample(0, None, 33, true, 1000),
                make_sample(33_000_000, None, 33, false, 1000),
                make_sample(66_000_000, None, 33, true, 1000),
            ],
        );

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
        let defaults = TrackDefaults::default();
        let chunk = make_chunk(0, vec![make_sample(0, Some(66_000_000), 33, true, 1000)]);
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        let entry = &truns[0].entries()[0];
        assert_eq!(entry.composition_time_offset, Some(5940));
    }

    #[test]
    fn build_truns_zero_cto_omitted() {
        let defaults = TrackDefaults::default();
        let chunk = make_chunk(0, vec![make_sample(0, Some(0), 33, true, 1000)]);
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert!(truns[0].entries()[0].composition_time_offset.is_none());
    }

    #[test]
    fn build_truns_multiple_chunks() {
        let defaults = TrackDefaults::default();
        let chunk1 = make_chunk(0, vec![make_sample(0, None, 33, true, 1000)]);
        let chunk2 = make_chunk(5000, vec![make_sample(33_000_000, None, 33, true, 2000)]);

        let table = make_sample_table(vec![chunk1, chunk2]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert_eq!(truns.len(), 2);
        assert_eq!(truns[0].data_offset(), Some(0));
        assert_eq!(truns[1].data_offset(), Some(5000));
    }

    #[test]
    fn build_truns_data_offset_set() {
        let defaults = TrackDefaults::default();
        let chunk = make_chunk(256, vec![make_sample(0, None, 33, true, 1000)]);
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert_eq!(truns[0].data_offset(), Some(256));
    }

    #[test]
    fn build_truns_duration_ticks_conversion() {
        let defaults = TrackDefaults::default();
        let chunk = make_chunk(0, vec![make_sample(0, None, 33, true, 1000)]);
        let table = make_sample_table(vec![chunk]);
        let truns = build_truns(&table, &defaults, &TickScale(nz(90000))).unwrap();

        assert_eq!(truns[0].entries()[0].duration, Some(2970));
    }

    // --- telescoping sum roundtrip ---

    fn ticks_to_nanos(ticks: u64, timescale: u64) -> u64 {
        let secs = ticks / timescale;
        let remainder = ticks % timescale;
        secs * 1_000_000_000 + remainder * 1_000_000_000 / timescale
    }

    #[test]
    fn telescoping_sum_consistency_uniform_delta() {
        let timescale: u64 = 15360;
        let delta_ticks: u64 = 512;
        let num_samples: u64 = 10;
        let scale = TickScale(nz(timescale as u32));

        let samples: Vec<_> = (0..num_samples)
            .map(|i| {
                let dts = ticks_to_nanos(i * delta_ticks, timescale);
                let next = ticks_to_nanos((i + 1) * delta_ticks, timescale);
                Sample::new(dts, None, Duration::from_nanos(next - dts), true, 100)
            })
            .collect();

        let total: u64 = samples
            .iter()
            .map(|s| u64::from(sample_delta_ticks(s.dts_ns, s.duration, &scale).unwrap()))
            .sum();

        let start_ns = samples.first().unwrap().dts_ns;
        let last = samples.last().unwrap();
        let end_ns = last.dts_ns + last.duration.as_nanos() as u64;
        let expected = scale.from_nanos(end_ns).unwrap() - scale.from_nanos(start_ns).unwrap();

        assert_eq!(total, expected);
    }

    #[test]
    fn telescoping_sum_consistency_variable_delta() {
        let timescale: u64 = 48000;
        let deltas: &[u64] = &[1024, 1024, 960, 1024, 1024, 960, 1024];
        let scale = TickScale(nz(timescale as u32));

        let mut dts_ticks: u64 = 0;
        let mut samples = Vec::with_capacity(deltas.len());
        for &dt in deltas {
            let dts = ticks_to_nanos(dts_ticks, timescale);
            let next = ticks_to_nanos(dts_ticks + dt, timescale);
            samples.push(Sample::new(
                dts,
                None,
                Duration::from_nanos(next - dts),
                true,
                100,
            ));
            dts_ticks += dt;
        }

        let total: u64 = samples
            .iter()
            .map(|s| u64::from(sample_delta_ticks(s.dts_ns, s.duration, &scale).unwrap()))
            .sum();

        let start_ns = samples.first().unwrap().dts_ns;
        let last = samples.last().unwrap();
        let end_ns = last.dts_ns + last.duration.as_nanos() as u64;
        let expected = scale.from_nanos(end_ns).unwrap() - scale.from_nanos(start_ns).unwrap();

        assert_eq!(total, expected);
    }

    #[test]
    fn telescoping_sum_consistency_media_duration() {
        let timescale: u64 = 15360;
        let delta_ticks: u64 = 512;
        let num_samples: u64 = 100;
        let scale = TickScale(nz(timescale as u32));

        let samples: Vec<_> = (0..num_samples)
            .map(|i| {
                let dts = ticks_to_nanos(i * delta_ticks, timescale);
                let next = ticks_to_nanos((i + 1) * delta_ticks, timescale);
                Sample::new(dts, None, Duration::from_nanos(next - dts), true, 100)
            })
            .collect();

        let table = make_sample_table(vec![make_chunk(0, samples)]);
        let duration = media_duration(&table, &scale).unwrap();
        let sum_deltas: u64 = table
            .chunks
            .iter()
            .flat_map(|c| c.samples.iter())
            .map(|s| u64::from(sample_delta_ticks(s.dts_ns, s.duration, &scale).unwrap()))
            .sum();

        assert_eq!(duration, sum_deltas);
    }

    #[test]
    fn telescoping_sum_exact_when_divisible() {
        let timescale: u64 = 1000;
        let delta_ticks: u64 = 33;
        let scale = TickScale(nz(timescale as u32));

        let samples: Vec<_> = (0..100u64)
            .map(|i| {
                let dts = ticks_to_nanos(i * delta_ticks, timescale);
                let next = ticks_to_nanos((i + 1) * delta_ticks, timescale);
                Sample::new(dts, None, Duration::from_nanos(next - dts), true, 100)
            })
            .collect();

        for sample in &samples {
            assert_eq!(
                u64::from(sample_delta_ticks(sample.dts_ns, sample.duration, &scale).unwrap()),
                delta_ticks
            );
        }
    }

    #[test]
    fn telescoping_sum_cross_timescale() {
        let media_ts: u64 = 15360;
        let movie_ts: u64 = 1000;
        let delta_ticks: u64 = 512;

        let samples: Vec<_> = (0..3u64)
            .map(|i| {
                let dts = ticks_to_nanos(i * delta_ticks, media_ts);
                let next = ticks_to_nanos((i + 1) * delta_ticks, media_ts);
                Sample::new(dts, None, Duration::from_nanos(next - dts), true, 100)
            })
            .collect();

        let table = make_sample_table(vec![make_chunk(0, samples)]);
        let movie_dur = media_duration(&table, &TickScale(nz(movie_ts as u32))).unwrap();
        assert_eq!(movie_dur, 100);
    }
}
