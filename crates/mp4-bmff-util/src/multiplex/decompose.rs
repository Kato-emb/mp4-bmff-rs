use mp4_bmff::BoxType;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::FourCC;

use super::Result;
use crate::multiplex::error::{Error, ErrorKind};

use crate::multiplex::container::{
    AudioSampleDescription, DataLayout, Movie, SampleDescription, Timeline, Timescale, Track,
    TrackId, VisualSampleDescription,
};

pub fn parse_movie(moov: &MoovBoxView<'_>) -> Result<Movie> {
    let mvhd = moov.mvhd()?;
    let timescale = Timescale::new(mvhd.timescale).ok_or(
        Error::new(ErrorKind::InvalidFormat)
            .with_message("Movie timescale must be a non-zero value"),
    )?;

    let mvex = moov.mvex()?;

    let tracks = moov
        .traks()
        .map(|trak| {
            let trak = trak?;
            let track_id = trak.tkhd()?.track_id;

            let trex = mvex.as_ref().and_then(|mvex| {
                mvex.trexs().find_map(|trex| {
                    if trex.as_ref().is_ok_and(|trex| trex.track_id == track_id) {
                        trex.ok()
                    } else {
                        None
                    }
                })
            });

            parse_track(&trak, trex.as_ref())
        })
        .collect::<Result<Vec<Track>>>()?;

    Ok(Movie { timescale, tracks })
}

fn parse_track(trak: &TrakBoxView<'_>, trex: Option<&TrexBox>) -> Result<Track> {
    let tkhd = trak.tkhd()?;
    let track_id = TrackId::new(tkhd.track_id)
        .ok_or(Error::new(ErrorKind::InvalidFormat).with_message("Invalid track ID"))?;

    let mdia = trak.mdia()?;
    let mdhd = mdia.mdhd()?;
    let timescale = Timescale::new(mdhd.timescale).ok_or(
        Error::new(ErrorKind::InvalidFormat)
            .with_message("Track timescale must be a non-zero value"),
    )?;
    let minf = mdia.minf()?;
    let hdlr = mdia.hdlr()?;
    let stbl = minf.stbl()?;

    let stsd = stbl.stsd()?;
    let descriptions = parse_sample_descriptions(hdlr.handler_type, &stsd)?;

    let timeline = parse_timeline(&stbl)?;

    let data_layouts = if trex.is_none() {
        alloc::vec![parse_data_layout(&stbl)?]
    } else {
        Vec::new()
    };

    let edit_list = match trak.edts()? {
        Some(edts) => {
            if let Some(elst) = edts.elst()? {
                let entries = elst.entries()?;
                Some(entries.collect())
            } else {
                None
            }
        }
        None => None,
    };

    Ok(Track {
        track_id,
        timescale,
        language: mdhd.language,
        matrix: tkhd.matrix,
        alternate_group: tkhd.alternate_group,
        descriptions,
        timeline,
        data_layouts,
        edit_list,
    })
}

fn parse_sample_descriptions(
    handler_type: FourCC,
    stsd: &StsdBoxView<'_>,
) -> Result<Vec<SampleDescription>> {
    match handler_type.as_ascii() {
        Some("vide") => parse_visual_descriptions(stsd),
        Some("soun") => parse_audio_descriptions(stsd),
        _ => Err(Error::new(ErrorKind::Unsupported).with_message(format!(
            "Unsupported handler type: {:?}",
            handler_type.as_ascii()
        ))),
    }
}

fn parse_visual_descriptions(stsd: &StsdBoxView<'_>) -> Result<Vec<SampleDescription>> {
    let mut descriptions = Vec::with_capacity(stsd.entry_count as usize);

    for entry in stsd.sample_entries() {
        let entry = entry?;
        match entry.boxtype() {
            #[cfg(feature = "avc")]
            BoxType::AVC1 => {
                use mp4_bmff::boxes::avc::Avc1SampleEntryView;
                let avc1 = entry.decode::<Avc1SampleEntryView<'_>>()?;
                let codec = VisualSampleDescription::Avc1(avc1.avcc()?.avc_config.to_owned());
                descriptions.push(SampleDescription::Video {
                    width: avc1.base().width,
                    height: avc1.base().height,
                    codec,
                });
            }
            #[cfg(feature = "avc")]
            BoxType::AVC3 => {
                use mp4_bmff::boxes::avc::Avc3SampleEntryView;
                let avc3 = entry.decode::<Avc3SampleEntryView<'_>>()?;
                let codec = VisualSampleDescription::Avc3(avc3.avcc()?.avc_config.to_owned());
                descriptions.push(SampleDescription::Video {
                    width: avc3.base().width,
                    height: avc3.base().height,
                    codec,
                });
            }
            #[cfg(feature = "mp4")]
            BoxType::MP4V => {
                use mp4_bmff::boxes::mp4::Mp4vSampleEntryView;
                let mp4v = entry.decode::<Mp4vSampleEntryView<'_>>()?;
                let codec = VisualSampleDescription::Mp4v(
                    mp4v.esds()?
                        .esd
                        .dec_config_descr()?
                        .dec_specific_info()?
                        .map(|dec| dec.instance().to_vec())
                        .unwrap_or_default(),
                );
                descriptions.push(SampleDescription::Video {
                    width: mp4v.base().width,
                    height: mp4v.base().height,
                    codec,
                });
            }
            _ => {
                return Err(Error::new(ErrorKind::Unsupported).with_message(format!(
                    "Unsupported sample entry type: {:?}",
                    entry.boxtype()
                )));
            }
        }
    }

    Ok(descriptions)
}

fn parse_audio_descriptions(stsd: &StsdBoxView<'_>) -> Result<Vec<SampleDescription>> {
    let mut descriptions = Vec::with_capacity(stsd.entry_count as usize);

    for entry in stsd.sample_entries() {
        let entry = entry?;
        match entry.boxtype() {
            #[cfg(feature = "mp4")]
            BoxType::MP4A => {
                use mp4_bmff::boxes::mp4::Mp4aSampleEntryView;
                let mp4a = entry.decode::<Mp4aSampleEntryView<'_>>()?;
                let codec = AudioSampleDescription::Mp4a(
                    mp4a.esds()?
                        .esd
                        .dec_config_descr()?
                        .dec_specific_info()?
                        .map(|dec| dec.instance().to_vec())
                        .unwrap_or_default(),
                );
                descriptions.push(SampleDescription::Audio {
                    channel_count: mp4a.base().channelcount,
                    sample_rate: mp4a.base().samplerate.integer() as u16,
                    codec,
                });
            }
            _ => {
                return Err(Error::new(ErrorKind::Unsupported).with_message(format!(
                    "Unsupported sample entry type: {:?}",
                    entry.boxtype()
                )));
            }
        }
    }

    Ok(descriptions)
}

fn parse_timeline(stbl: &StblBoxView<'_>) -> Result<Timeline> {
    let stts_entries = stbl.stts()?.entries().collect();
    let ctts_entries = stbl.ctts()?.map(|ctts| ctts.entries().collect());
    let stss_entries = stbl.stss()?.map(|stss| stss.entries().collect());
    let sdtp_entries = stbl.sdtp()?.map(|sdtp| sdtp.entries().collect());

    Ok(Timeline {
        stts_entries,
        ctts_entries,
        stss_entries,
        sdtp_entries,
    })
}

fn parse_data_layout(stbl: &StblBoxView<'_>) -> Result<DataLayout> {
    let sample_sizes = if let Some(stsz) = stbl.stsz()? {
        stsz.entries().map(|e| e.entry_size).collect()
    } else if let Some(stz2) = stbl.stz2()? {
        stz2.entries().map(|e| u32::from(e.entry_size)).collect()
    } else {
        return Err(Error::new(ErrorKind::InvalidFormat)
            .with_message("Track must contain either stsz or stz2 box for sample sizes"));
    };

    let stsc_entries = stbl.stsc()?.entries().collect();

    let chunk_offsets = if let Some(stco) = stbl.stco()? {
        stco.entries().map(|e| u64::from(e.chunk_offset)).collect()
    } else if let Some(co64) = stbl.co64()? {
        co64.entries().map(|e| e.chunk_offset).collect()
    } else {
        return Err(Error::new(ErrorKind::InvalidFormat)
            .with_message("Track must contain either stco or co64 box for chunk offsets"));
    };

    Ok(DataLayout {
        sample_sizes,
        stsc_entries,
        chunk_offsets,
    })
}

fn parse_fragment(
    traf: &TrafBoxView<'_>,
    moof_offset: u64,
    trexs: &[TrexBox],
) -> Result<(TrackId, Timeline, DataLayout)> {
    let tfhd = traf.tfhd()?;

    let track_id = TrackId::new(tfhd.track_id)
        .ok_or(Error::new(ErrorKind::InvalidFormat).with_message("Invalid track ID in tfhd"))?;

    let trex = trexs
        .iter()
        .find(|trex| trex.track_id == tfhd.track_id)
        .ok_or(Error::new(ErrorKind::InvalidFormat).with_message(format!(
            "No matching trex box found for track ID {}",
            tfhd.track_id
        )))?;

    let tfdt = traf.tfdt()?;

    let _base_media_decode_time = tfdt
        .as_ref()
        .map(|tfdt| tfdt.base_media_decode_time)
        .unwrap_or(0);

    let base_data_offset = if tfhd.flags.contains(TfhdFlags::DEFAULT_BASE_IS_MOOF) {
        moof_offset
    } else {
        tfhd.base_data_offset
            .ok_or(Error::new(ErrorKind::InvalidFormat).with_message(
                "tfhd must contain base_data_offset if default_base_is_moof flag is not set",
            ))?
    };

    let mut all_stts: Vec<SttsEntry> = Vec::new();
    let mut all_ctts: Option<Vec<CttsEntry>> = None;
    let mut all_stss: Option<Vec<StssEntry>> = None;
    // let mut all_sdtp: Option<Vec<SdtpEntry>> = None;

    let mut sample_sizes = Vec::new();
    let mut stsc_entries = Vec::new();
    let mut chunk_offsets = Vec::new();

    let mut sample_index_base: u32 = 0;
    let mut running_data_offset = base_data_offset;
    let mut has_any_non_sync = false;

    for trun in traf.truns() {
        let trun = trun?;

        let trun_data_offset = if let Some(offset) = trun.data_offset {
            (base_data_offset as i64).saturating_add(offset as i64) as u64
        } else {
            running_data_offset
        };

        chunk_offsets.push(trun_data_offset);
        stsc_entries.push(StscEntry {
            first_chunk: chunk_offsets.len() as u32,
            samples_per_chunk: trun.sample_count,
            sample_description_index: tfhd
                .sample_description_index
                .unwrap_or(trex.default_sample_description_index),
        });

        let mut chunk_byte_size: u64 = 0;

        for (i, entry) in trun.entries().enumerate() {
            let entry = entry?;

            let duration = entry
                .duration
                .or(tfhd.default_sample_duration)
                .unwrap_or(trex.default_sample_duration);

            let sample_size = entry
                .size
                .or(tfhd.default_sample_size)
                .unwrap_or(trex.default_sample_size);

            let flags = entry
                .flags
                .or_else(|| {
                    if i == 0 {
                        trun.first_sample_flags
                    } else {
                        None
                    }
                })
                .or(tfhd.default_sample_flags)
                .unwrap_or(trex.default_sample_flags);
            let cts_offset = entry.composition_time_offset;

            accumulate_stts(&mut all_stts, duration);

            if let Some(offset) = cts_offset {
                let ctts = all_ctts.get_or_insert_with(|| {
                    // 初めてcttsが必要になった時点で、
                    // 先行サンプル分（sample_index_base + i個）をoffset=0で埋める
                    let preceding = sample_index_base as usize + i;
                    if preceding > 0 {
                        vec![CttsEntry {
                            sample_count: preceding as u32,
                            sample_offset: 0,
                        }]
                    } else {
                        Vec::new()
                    }
                });
                ctts.push(CttsEntry {
                    sample_count: 1,
                    sample_offset: i32::try_from(offset)
                        .map_err(|e| Error::new(ErrorKind::Overflow).with_source(e))?,
                });
            } else if all_ctts.is_some() {
                // cttsが既に存在するなら、offset=0で埋める
                all_ctts.as_mut().unwrap().push(CttsEntry {
                    sample_count: 1,
                    sample_offset: 0,
                });
            }

            let is_sync = flags.is_sync();

            if !is_sync {
                has_any_non_sync = true;
            }

            if is_sync {
                let global_index = sample_index_base + i as u32 + 1; // stss is 1-based
                all_stss.get_or_insert_with(Vec::new).push(StssEntry {
                    sample_number: global_index,
                });
            }

            // TODO: sdtp flags parsing

            sample_sizes.push(sample_size);
            chunk_byte_size = chunk_byte_size.checked_add(u64::from(sample_size)).ok_or(
                Error::new(ErrorKind::Overflow).with_message("Chunk byte size exceeds u64 maximum"),
            )?;
        }

        running_data_offset = running_data_offset.checked_add(chunk_byte_size).ok_or(
            Error::new(ErrorKind::Overflow).with_message(
                "Running data offset exceeds u64 maximum after adding chunk byte size",
            ),
        )?;
        sample_index_base = sample_index_base.checked_add(trun.sample_count).ok_or(
            Error::new(ErrorKind::Overflow).with_message(
                "Sample index base exceeds u32 maximum after adding trun sample count",
            ),
        )?;
    }

    if !has_any_non_sync {
        // If all samples in the chunk are sync samples, we can omit stss entries for this chunk
        all_stss = None;
    }

    let timeline = Timeline {
        stts_entries: all_stts,
        ctts_entries: all_ctts,
        stss_entries: all_stss,
        sdtp_entries: None, // TODO
    };

    let data_layout = DataLayout {
        sample_sizes,
        stsc_entries,
        chunk_offsets,
    };

    Ok((track_id, timeline, data_layout))
}

fn accumulate_stts(entries: &mut Vec<SttsEntry>, duration: u32) {
    if let Some(last) = entries.last_mut().filter(|e| e.sample_delta == duration) {
        last.sample_count += 1;
    } else {
        entries.push(SttsEntry {
            sample_count: 1,
            sample_delta: duration,
        });
    }
}
