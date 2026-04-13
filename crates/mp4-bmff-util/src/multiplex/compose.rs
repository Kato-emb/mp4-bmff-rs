use mp4_bmff::RawBoxOwned;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::U8F8;
use mp4_bmff::types::U16F16;

use super::container::Movie;
use super::container::Track;
use super::container::{AudioSampleDescription, SampleDescription, VisualSampleDescription};
use crate::multiplex::Result;
use crate::multiplex::container::DataLayout;
use crate::multiplex::container::Timeline;
use crate::multiplex::error::{Error, ErrorKind};

fn compose_moov(movie: &Movie) -> Result<MoovBox> {
    todo!()
}

fn compose_trak(track: &Track) -> Result<TrakBox> {
    todo!()
}

fn compose_tkhd(track: &Track) -> Result<TkhdBox> {
    let duration = track
        .edit_duration()
        .unwrap_or(track.timeline.media_duration());

    let mut tkhd = TkhdBox::new(track.track_id.as_nonzero(), duration);
    tkhd.alternate_group = track.alternate_group;
    tkhd.matrix = track.matrix;

    match track.descriptions.first() {
        Some(SampleDescription::Video { width, height, .. }) => {
            tkhd.width = U16F16::from_integer(i128::from(*width));
            tkhd.height = U16F16::from_integer(i128::from(*height));
        }
        Some(SampleDescription::Audio { .. }) => tkhd.volume = U8F8::from_f32(1.0),
        _ => {}
    }

    Ok(tkhd)
}

fn compose_mdia(track: &Track) -> Result<MdiaBox> {
    Ok(MdiaBox {
        mdhd: compose_mdhd(track),
        hdlr: compose_hdlr(&track.descriptions)?,
        minf: compose_minf(track)?,
        elng: None,
    })
}

fn compose_mdhd(track: &Track) -> MdhdBox {
    let media_duration = track.timeline.media_duration();

    MdhdBox {
        timescale: track.timescale.as_u32(),
        duration: media_duration,
        language: track.language,
        ..Default::default()
    }
}

fn compose_hdlr(descs: &[SampleDescription]) -> Result<HdlrBox> {
    let handler_type = match descs.first() {
        Some(desc) => desc.handler_type(),
        None => return Err(Error::new(ErrorKind::InvalidInput)),
    };

    Ok(HdlrBox::new(handler_type))
}

fn compose_minf(track: &Track) -> Result<MinfBox> {
    let stsd = compose_stsd(&track.descriptions)?;

    todo!()
}

fn compose_media_header(descs: &[SampleDescription]) -> MediaHeaderBox {
    match descs.first() {
        Some(SampleDescription::Video { .. }) => MediaHeaderBox::Vmhd(VmhdBox::default()),
        Some(SampleDescription::Audio { .. }) => MediaHeaderBox::Smhd(SmhdBox::default()),
        Some(SampleDescription::Hint { .. }) => MediaHeaderBox::Hmhd(HmhdBox::default()),
        _ => MediaHeaderBox::Nmhd(NmhdBox::default()),
    }
}

fn compose_stsd(descs: &[SampleDescription]) -> Result<StsdBox> {
    let entries = descs
        .iter()
        .map(build_sample_entry)
        .collect::<Result<Vec<_>>>()?;

    Ok(StsdBox {
        entries,
        ..Default::default()
    })
}

fn build_sample_entry(media: &SampleDescription) -> Result<RawBoxOwned> {
    match media {
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
            RawBoxOwned::from_boxed(&avc1).map_err(Into::into)
        }
        #[cfg(feature = "avc")]
        VisualSampleDescription::Avc3(avc_config) => {
            use mp4_bmff::boxes::avc::Avc3SampleEntry;
            use mp4_bmff::boxes::avc::AvcCBox;
            let avcc = AvcCBox {
                avc_config: avc_config.clone(),
            };

            let avc3 = Avc3SampleEntry::new(base, avcc, None, None);
            RawBoxOwned::from_boxed(&avc3).map_err(Into::into)
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
            RawBoxOwned::from_boxed(&mp4v).map_err(Into::into)
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
            RawBoxOwned::from_boxed(&mp4a).map_err(Into::into)
        }
    }
}

fn compose_stbl(timeline: Timeline, data_layout: DataLayout, stsd: StsdBox) -> StblBox {
    let (stts, sample_size, ctts, stss, sdtp) = compose_timeline(timeline);
    let (stsc, chunk_offset) = compose_data_layout(data_layout);

    StblBox {
        stsd,
        stts,
        sample_size,
        ctts,
        stss,
        sdtp,
        stsc,
        chunk_offset,
        cslg: None,
        stdp: None,
        stsh: None,
        sbgps: Vec::new(),
        sgpds: Vec::new(),
    }
}

fn compose_timeline(
    timeline: Timeline,
) -> (
    SttsBox,
    SampleSize,
    Option<CttsBox>,
    Option<StssBox>,
    Option<SdtpBox>,
) {
    let stts = SttsBox {
        entries: timeline.stts_entries,
        ..Default::default()
    };

    let sample_size = compress_stsz(timeline.sample_sizes);

    let ctts = if let Some(ctts_entries) = timeline.ctts_entries {
        Some(CttsBox {
            entries: ctts_entries,
            ..Default::default()
        })
    } else {
        None
    };

    let stss = if let Some(stss_entries) = timeline.stss_entries {
        Some(StssBox {
            entries: stss_entries,
            ..Default::default()
        })
    } else {
        None
    };

    let sdtp = if let Some(sdtp_entries) = timeline.sdtp_entries {
        Some(SdtpBox {
            entries: sdtp_entries,
            ..Default::default()
        })
    } else {
        None
    };

    (stts, sample_size, ctts, stss, sdtp)
}

fn compose_data_layout(data_layout: DataLayout) -> (StscBox, ChunkOffset) {
    let stsc = StscBox {
        entries: data_layout.stsc_entries,
        ..Default::default()
    };
    let chunk_offset = compress_chunk_offset(data_layout.chunk_offsets);

    (stsc, chunk_offset)
}

fn compress_stsz(sample_sizes: Vec<StszEntry>) -> SampleSize {
    if sample_sizes.is_empty() {
        SampleSize::Stsz(StszBox {
            sample_size: 0,
            sample_count: 0,
            entries: Vec::new(),
            ..Default::default()
        })
    } else if sample_sizes
        .iter()
        .all(|size| size.entry_size == sample_sizes[0].entry_size)
    {
        SampleSize::Stsz(StszBox {
            sample_size: sample_sizes[0].entry_size,
            sample_count: sample_sizes.len() as u32,
            entries: Vec::new(),
            ..Default::default()
        })
    } else {
        SampleSize::Stsz(StszBox {
            sample_size: 0,
            sample_count: sample_sizes.len() as u32,
            entries: sample_sizes
                .into_iter()
                .map(|size| StszEntry {
                    entry_size: size.entry_size,
                })
                .collect(),
            ..Default::default()
        })
    }
}

fn compress_chunk_offset(chunk_offsets: Vec<u64>) -> ChunkOffset {
    if chunk_offsets.is_empty() {
        ChunkOffset::Stco(StcoBox {
            entries: Vec::new(),
            ..Default::default()
        })
    } else if chunk_offsets
        .iter()
        .all(|&offset| offset <= u32::MAX as u64)
    {
        ChunkOffset::Stco(StcoBox {
            entries: chunk_offsets
                .into_iter()
                .map(|offset| StcoEntry {
                    chunk_offset: offset as u32,
                })
                .collect(),
            ..Default::default()
        })
    } else {
        ChunkOffset::Co64(Co64Box {
            entries: chunk_offsets
                .into_iter()
                .map(|offset| Co64Entry {
                    chunk_offset: offset,
                })
                .collect(),
            ..Default::default()
        })
    }
}

fn compose_edit_list(edit_list: Vec<ElstEntry>) -> Option<EdtsBox> {
    if edit_list.is_empty() {
        Some(EdtsBox { elst: None })
    } else {
        Some(EdtsBox {
            elst: Some(ElstBox {
                entries: edit_list,
                ..Default::default()
            }),
        })
    }
}
