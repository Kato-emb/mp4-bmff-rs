use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::RawBoxOwned;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::{I16F16, U8F8, U16F16};

use crate::multiplex::Result;
use crate::multiplex::error::*;
use crate::multiplex::repr::*;

use crate::multiplex::Chunk;
use crate::multiplex::EditSegment;
use crate::multiplex::Sample;
use crate::multiplex::Track;
use crate::multiplex::{
    AudioSampleDescription, //
    MediaDefinition,
    VisualSampleDescription,
};

impl TickScale {
    /// Converts nanoseconds to ticks.
    pub(super) fn nanos_to_ticks(&self, nanos: u64) -> Option<u64> {
        let ts = u64::from(self.0.get());
        let secs = nanos / 1_000_000_000;
        let sub_nanos = nanos % 1_000_000_000;

        secs.checked_mul(ts)?
            .checked_add(sub_nanos * ts / 1_000_000_000)
    }

    /// Converts a `Duration` to ticks.
    pub(super) fn duration_to_ticks(&self, duration: Duration) -> Option<u64> {
        let ts = u64::from(self.0.get());
        let secs = duration.as_secs();
        let sub_nanos = u64::from(duration.subsec_nanos());

        secs.checked_mul(ts)?
            .checked_add(sub_nanos * ts / 1_000_000_000)
    }
}

// ---------------------------------------------------------------------------
// Domain helpers — bridge between domain types and TickScale
// ---------------------------------------------------------------------------
fn media_duration_ticks(chunks: &[Chunk], scale: &TickScale) -> Option<u64> {
    let Some(start_ns) = chunks.first().map(|c| c.first_sample().dts_ns()) else {
        return Some(0);
    };

    let Some(last_sample) = chunks.last().map(|c| c.last_sample()) else {
        return Some(0);
    };

    let end_ns = last_sample.dts_ns() + u64::try_from(last_sample.duration().as_nanos()).ok()?;

    scale
        .nanos_to_ticks(end_ns)?
        .checked_sub(scale.nanos_to_ticks(start_ns)?)
}

fn track_duration_ticks(track_repr: &TrackRepr, scale: &TickScale) -> Option<u64> {
    if let Some(edit_duration) = track_repr.track.edit_duration() {
        scale.duration_to_ticks(edit_duration)
    } else {
        media_duration_ticks(&track_repr.chunks, scale)
    }
}

fn sample_delta_ticks(sample: &Sample, scale: &TickScale) -> Option<u32> {
    let start_ns = sample.dts_ns();
    let end_ns = u64::try_from(sample.duration().as_nanos())
        .ok()?
        .checked_add(start_ns)?;

    scale
        .nanos_to_ticks(end_ns)?
        .checked_sub(scale.nanos_to_ticks(start_ns)?)
        .and_then(|ticks| u32::try_from(ticks).ok())
}

fn sample_cto_ticks(sample: &Sample, scale: &TickScale) -> Option<i64> {
    let dts_ns = sample.dts_ns();
    let Some(pts_ns) = sample.pts_ns() else {
        return Some(0);
    };

    let (abs_cto_ns, negative) = if pts_ns >= 0 {
        let pts_uns = pts_ns as u64;
        if pts_uns >= dts_ns {
            (pts_uns - dts_ns, false)
        } else {
            (dts_ns - pts_uns, true)
        }
    } else {
        (dts_ns.checked_add(pts_ns.unsigned_abs())?, true)
    };

    let ticks = i64::try_from(scale.nanos_to_ticks(abs_cto_ns)?).ok()?;
    Some(if negative { -ticks } else { ticks })
}

// ---------------------------------------------------------------------------
// Non-fragmented: moov
// ---------------------------------------------------------------------------

pub(super) fn build_moov(movie: &Movie) -> Result<MoovBox> {
    let mvhd = build_mvhd(movie)?;

    let traks = movie
        .tracks
        .iter()
        .map(|track_repr| build_trak(track_repr, movie.timescale))
        .collect::<Result<Vec<_>>>()?;

    Ok(MoovBox {
        mvhd,
        traks,
        mvex: None,
    })
}

fn build_mvhd(movie: &Movie) -> Result<MvhdBox> {
    let movie_scale = TickScale(movie.timescale);
    let duration = movie
        .tracks
        .iter()
        .filter_map(|track_repr| track_duration_ticks(track_repr, &movie_scale))
        .max()
        .unwrap_or(0);

    Ok(MvhdBox {
        timescale: movie.timescale.get(),
        duration,
        next_track_id: movie.next_track_id,
        ..Default::default()
    })
}

fn build_trak(track_repr: &TrackRepr, movie_timescale: NonZeroU32) -> Result<TrakBox> {
    Ok(TrakBox {
        tkhd: build_tkhd(track_repr, movie_timescale)?,
        mdia: build_mdia(track_repr)?,
        edts: build_edts(&track_repr.track, movie_timescale)?,
        tref: None,
        trgr: None,
    })
}

fn build_tkhd(track_repr: &TrackRepr, movie_timescale: NonZeroU32) -> Result<TkhdBox> {
    let movie_scale = TickScale(movie_timescale);
    let track_duration = track_duration_ticks(track_repr, &movie_scale)
        .ok_or(Error::new(ErrorKind::Overflow).with_message(
            "Track duration exceeds maximum representable value in track timescale",
        ))?;
    let mut tkhd = TkhdBox::new(track_repr.track.id().into_inner(), track_duration);
    tkhd.alternate_group = track_repr.track.alternate_group();
    tkhd.matrix = track_repr.track.matrix();

    match track_repr.track.media() {
        MediaDefinition::Video { width, height, .. } => {
            tkhd.width = U16F16::from_integer(i128::from(*width));
            tkhd.height = U16F16::from_integer(i128::from(*height));
        }
        MediaDefinition::Audio { .. } => tkhd.volume = U8F8::from_f32(1.0),
        _ => {}
    }

    Ok(tkhd)
}

fn build_mdia(track_repr: &TrackRepr) -> Result<MdiaBox> {
    Ok(MdiaBox {
        mdhd: build_mdhd(track_repr, track_repr.track.timescale)?,
        hdlr: HdlrBox::new(track_repr.track.media().handler_type()),
        minf: build_minf(track_repr)?,
        elng: None,
    })
}

fn build_mdhd(track_repr: &TrackRepr, media_timescale: NonZeroU32) -> Result<MdhdBox> {
    let media_scale = TickScale(media_timescale);
    let media_duration = media_duration_ticks(&track_repr.chunks, &media_scale)
        .ok_or(Error::new(ErrorKind::Overflow).with_message(
            "Media duration exceeds maximum representable value in track timescale",
        ))?;

    Ok(MdhdBox::new(
        track_repr.track.timescale,
        media_duration,
        track_repr.track.language,
    ))
}

fn build_minf(track_repr: &TrackRepr) -> Result<MinfBox> {
    let stsd = build_stsd(&track_repr.track)?;

    Ok(MinfBox {
        media_header: build_media_header(&track_repr.track),
        stbl: build_stbl(track_repr, stsd)?,
        dinf: DinfBox::self_contained(),
    })
}

fn build_media_header(track: &Track) -> MediaHeaderBox {
    match track.media() {
        MediaDefinition::Video { .. } => MediaHeaderBox::Vmhd(VmhdBox::default()),
        MediaDefinition::Audio { .. } => MediaHeaderBox::Smhd(SmhdBox::default()),
        MediaDefinition::Hint { .. } => MediaHeaderBox::Hmhd(HmhdBox::default()),
        _ => MediaHeaderBox::Nmhd(NmhdBox::default()),
    }
}

fn build_stsd(track: &Track) -> Result<StsdBox> {
    let sample_entry = build_sample_entry(track.media())?;

    Ok(StsdBox {
        entries: alloc::vec![sample_entry],
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
        #[cfg(feature = "avc")]
        VisualSampleDescription::Avc1(avc_config) => {
            use mp4_bmff::boxes::avc::Avc1SampleEntry;
            use mp4_bmff::boxes::avc::AvcCBox;
            let avcc = AvcCBox {
                avc_config: avc_config.clone(),
            };

            let avc1 = Avc1SampleEntry::new(base, avcc, None, None);
            RawBoxOwned::from_boxed(&avc1).map_err(|e| Error::box_encode(e))
        }
        #[cfg(feature = "avc")]
        VisualSampleDescription::Avc3(avc_config) => {
            use mp4_bmff::boxes::avc::Avc3SampleEntry;
            use mp4_bmff::boxes::avc::AvcCBox;
            let avcc = AvcCBox {
                avc_config: avc_config.clone(),
            };

            let avc3 = Avc3SampleEntry::new(base, avcc, None, None);
            RawBoxOwned::from_boxed(&avc3).map_err(|e| Error::box_encode(e))
        }
        #[cfg(feature = "mp4")]
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
        #[cfg(feature = "mp4")]
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

fn build_stbl(track_repr: &TrackRepr, stsd: StsdBox) -> Result<StblBox> {
    let mut stts = SttsBox::default();
    let mut stsz = StszBox::default();
    let mut stsc = StscBox::default();
    let mut ctts = CttsBox::default();
    let mut stss = StssBox::default();
    let mut chunk_offset = ChunkOffset::default();

    let mut has_nonzero_ctts = false;
    let mut sample_number: u32 = 0;

    let media_scale = TickScale(track_repr.track.timescale);

    for (chunk_index, chunk) in track_repr.chunks.iter().enumerate() {
        let samples_per_chunk = u32::try_from(chunk.sample_count()).map_err(|_| {
            Error::new(ErrorKind::Overflow)
                .with_message("Number of samples in chunk exceeds u32 limit")
        })?;

        stsc.push(chunk_index as u32 + 1, samples_per_chunk, 1);
        chunk_offset.push(chunk.data_offset());

        for sample in chunk.samples() {
            sample_number = sample_number.checked_add(1).ok_or(ErrorKind::Overflow)?;

            let delta = sample_delta_ticks(sample, &media_scale).ok_or(ErrorKind::Overflow)?;
            let cto = sample_cto_ticks(sample, &media_scale).ok_or(ErrorKind::Overflow)?;

            stts.push(delta);
            stsz.push(sample.size());
            ctts.push(i32::try_from(cto).map_err(|_| {
                Error::new(ErrorKind::Overflow)
                    .with_message("Sample composition time offset exceeds i32 tick range")
            })?);
            has_nonzero_ctts |= cto != 0;
            if sample.is_sync() {
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
    let Some(segments) = track.edit_list() else {
        return Ok(None);
    };

    let movie_scale = TickScale(movie_timescale);
    let media_scale = TickScale(track.timescale);

    let mut entries = Vec::with_capacity(segments.len());

    for seg in segments {
        let entry = match seg {
            EditSegment::Empty { duration } => ElstEntry {
                segment_duration: movie_scale
                    .duration_to_ticks(*duration)
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
                    .duration_to_ticks(*duration)
                    .ok_or(ErrorKind::Overflow)?,
                media_time: media_scale
                    .duration_to_ticks(*media_start)
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
                    .duration_to_ticks(*duration)
                    .ok_or(ErrorKind::Overflow)?,
                media_time: media_scale
                    .duration_to_ticks(*media_time)
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
