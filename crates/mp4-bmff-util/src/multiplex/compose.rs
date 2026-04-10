use mp4_bmff::RawBoxOwned;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::U8F8;
use mp4_bmff::types::U16F16;

use super::Result;

use super::Context;
use super::DataLayout;

use super::AudioSampleDescription;
use super::SampleDescription;
use super::VisualSampleDescription;
use super::error::*;
use super::repr::*;

fn movie_duration(movie: &Movie) -> Option<u64> {
    movie.tracks.iter().try_fold(None::<u64>, |acc, tr| {
        let duration = track_duration(tr, movie.timescale)?;
        Some(Some(acc.map_or(duration, |a: u64| a.max(duration))))
    })?
}

fn track_duration(track: &Track, movie_timescale: Timescale) -> Option<u64> {
    let media_duration = track
        .timescale
        .rescale_ticks(track.media_duration(), movie_timescale);
    track.edit_duration().or(media_duration)
}

/// Builds a `MoovBox` from the given `Context` and `Layout`, representing the movie metadata and track layout for an MP4 file.
pub fn build_moov(context: &Context, layout: &DataLayout) -> Result<MoovBox> {
    let mvhd = build_mvhd(&context.movie)?;

    let traks = context
        .movie
        .tracks
        .iter()
        .map(|tr| build_trak(tr, layout, context.movie.timescale))
        .collect::<Result<Vec<_>>>()?;

    Ok(MoovBox {
        mvhd,
        traks,
        mvex: None,
    })
}

fn build_mvhd(movie: &Movie) -> Result<MvhdBox> {
    let movie_timescale = movie.timescale;
    let duration = movie_duration(movie).ok_or(Error::new(ErrorKind::Overflow).with_message(
        "Movie duration exceeds representable range for mvhd box (2^64 - 1 ticks)",
    ))?;
    let next_track_id = movie
        .next_track_id()
        .ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Exceeded maximum number of tracks (2^32 - 1)"),
        )?
        .as_u32();

    Ok(MvhdBox {
        timescale: movie_timescale.as_u32(),
        duration,
        next_track_id,
        ..Default::default()
    })
}

fn build_trak(track: &Track, layout: &DataLayout, movie_timescale: Timescale) -> Result<TrakBox> {
    Ok(TrakBox {
        tkhd: build_tkhd(track, movie_timescale)?,
        mdia: build_mdia(track, layout)?,
        edts: build_edts(track)?,
        tref: None,
        trgr: None,
    })
}

fn build_tkhd(track: &Track, movie_timescale: Timescale) -> Result<TkhdBox> {
    let track_duration = track_duration(track, movie_timescale).ok_or(
        Error::new(ErrorKind::Overflow).with_message(
            "Track duration exceeds representable range for tkhd box (2^64 - 1 ticks)",
        ),
    )?;
    let mut tkhd = TkhdBox::new(track.id.as_nonzero(), track_duration);
    tkhd.alternate_group = track.alternate_group;
    tkhd.matrix = track.matrix;

    match track.primary_description() {
        Some(SampleDescription::Video { width, height, .. }) => {
            tkhd.width = U16F16::from_integer(i128::from(*width));
            tkhd.height = U16F16::from_integer(i128::from(*height));
        }
        Some(SampleDescription::Audio { .. }) => tkhd.volume = U8F8::from_f32(1.0),
        _ => {}
    }

    Ok(tkhd)
}

fn build_mdia(track: &Track, layout: &DataLayout) -> Result<MdiaBox> {
    Ok(MdiaBox {
        mdhd: build_mdhd(track),
        hdlr: build_hdlr(track)?,
        minf: build_minf(track, layout)?,
        elng: None,
    })
}

fn build_mdhd(track: &Track) -> MdhdBox {
    MdhdBox {
        timescale: track.timescale.as_u32(),
        duration: track.media_duration(),
        language: track.language,
        ..Default::default()
    }
}

fn build_hdlr(track: &Track) -> Result<HdlrBox> {
    let handler_type = track
        .primary_description()
        .map(|desc| desc.handler_type())
        .ok_or(Error::new(ErrorKind::InvalidFormat))?;
    Ok(HdlrBox::new(handler_type))
}

fn build_minf(track: &Track, layout: &DataLayout) -> Result<MinfBox> {
    Ok(MinfBox {
        media_header: build_media_header(track),
        stbl: build_stbl(track, layout)?,
        dinf: DinfBox::self_contained(),
    })
}

fn build_media_header(track: &Track) -> MediaHeaderBox {
    match track.primary_description() {
        Some(SampleDescription::Video { .. }) => MediaHeaderBox::Vmhd(VmhdBox::default()),
        Some(SampleDescription::Audio { .. }) => MediaHeaderBox::Smhd(SmhdBox::default()),
        Some(SampleDescription::Hint { .. }) => MediaHeaderBox::Hmhd(HmhdBox::default()),
        _ => MediaHeaderBox::Nmhd(NmhdBox::default()),
    }
}

fn build_stbl(track: &Track, layout: &DataLayout) -> Result<StblBox> {
    let stsd = build_stsd(track)?;

    let mut stts = SttsBox::default();
    let mut stsz = StszBox::default();
    let mut stsc = StscBox::default();
    let mut ctts = CttsBox::default();
    let mut stss = StssBox::default();
    let mut chunk_offset = ChunkOffset::default();

    let mut has_nonzero_ctts = false;
    let mut sample_number: usize = 0;

    for (index, chunk) in layout.chunks_for_track(track.id).enumerate() {
        let samples_per_chunk = chunk.num_samples();
        stsc.push(index as u32 + 1, samples_per_chunk as u32, 1);
        chunk_offset.push(chunk.base_offset());

        let samples = track
            .samples
            .get(sample_number..sample_number + samples_per_chunk)
            .ok_or(Error::new(ErrorKind::__Unknown))?;

        for (sample, sample_size) in samples.iter().zip(chunk.sizes()) {
            sample_number += 1;

            stts.push(sample.delta);
            stsz.push(*sample_size);

            ctts.push(i32::try_from(sample.composition_time_offset).map_err(|_| {
                Error::new(ErrorKind::Overflow)
                    .with_message("Sample composition time offset exceeds i32 tick range")
            })?);

            if sample.is_sync {
                stss.push(sample_number as u32);
            }

            if sample.composition_time_offset != 0 {
                has_nonzero_ctts = true;
            }
        }
    }

    // Omit ctts if all offsets are zero.
    let ctts = if has_nonzero_ctts { Some(ctts) } else { None };
    // Omit stss if every sample is a sync sample (ISO 14496-12 §8.6.2).
    let stss = if stss.entries.len() == sample_number {
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

fn build_stsd(track: &Track) -> Result<StsdBox> {
    let entries = build_sample_entry(&track.descriptions)?;
    Ok(StsdBox {
        entries,
        ..Default::default()
    })
}

fn build_sample_entry(descs: &[SampleDescription]) -> Result<Vec<RawBoxOwned>> {
    let mut entries = Vec::with_capacity(descs.len());

    for desc in descs {
        let entry = match desc {
            SampleDescription::Video {
                width,
                height,
                codec,
            } => build_visual_sample_entry(*width, *height, codec),
            SampleDescription::Audio {
                channel_count,
                sample_rate,
                codec,
            } => build_audio_sample_entry(*channel_count, *sample_rate, codec),
            _ => Err(Error::new(ErrorKind::Unsupported)
                .with_message("Unsupported media definition for sample entry construction")),
        }?;

        entries.push(entry);
    }

    Ok(entries)
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

    let result = match codec {
        #[cfg(feature = "avc")]
        VisualSampleDescription::Avc1(avc_config) => {
            use mp4_bmff::boxes::avc::Avc1SampleEntry;
            use mp4_bmff::boxes::avc::AvcCBox;
            let avcc = AvcCBox {
                avc_config: avc_config.clone(),
            };

            let avc1 = Avc1SampleEntry::new(base, avcc, None, None);
            RawBoxOwned::from_boxed(&avc1)
        }
        #[cfg(feature = "avc")]
        VisualSampleDescription::Avc3(avc_config) => {
            use mp4_bmff::boxes::avc::Avc3SampleEntry;
            use mp4_bmff::boxes::avc::AvcCBox;
            let avcc = AvcCBox {
                avc_config: avc_config.clone(),
            };

            let avc3 = Avc3SampleEntry::new(base, avcc, None, None);
            RawBoxOwned::from_boxed(&avc3)
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
            RawBoxOwned::from_boxed(&mp4v)
        }
    };

    result.map_err(|e| Error::new(ErrorKind::Encode).with_source(e))
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

    let result = match codec {
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
            RawBoxOwned::from_boxed(&mp4a)
        }
    };

    result.map_err(|e| Error::new(ErrorKind::Encode).with_source(e))
}

fn build_edts(track: &Track) -> Result<Option<EdtsBox>> {
    build_elst(track).map(|elst| elst.map(|e| EdtsBox { elst: Some(e) }))
}

fn build_elst(track: &Track) -> Result<Option<ElstBox>> {
    let Some(edits) = &track.edit_list else {
        return Ok(None);
    };

    let entries = edits
        .iter()
        .map(|e| ElstEntry {
            segment_duration: e.segment_duration,
            media_time: e.media_time,
            media_rate: e.media_rate,
        })
        .collect();

    Ok(Some(ElstBox::new(entries)))
}

// fn build_moof(context: &Context, layout: &Layout, sequence_number: u32) -> Result<MoofBox> {
//     todo!()
// }

// fn build_mfhd(sequence_number: u32) -> MfhdBox {
//     MfhdBox::new(sequence_number)
// }

// fn build_traf() -> Result<TrafBox> {
//     todo!()
// }

// fn build_tfhd() -> Result<TfhdBox> {
//     // TODO: デフォルト値は推奨値を算出する
//     todo!()
// }

// fn build_tfdt() -> Result<TfdtBox> {
//     // TODO: bmdt はステートフルな値になるので、引数で渡す？
//     todo!()
// }

// fn build_trun() -> Result<TrunBox> {
//     todo!()
// }
