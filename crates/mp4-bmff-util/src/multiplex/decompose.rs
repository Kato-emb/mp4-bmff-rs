use mp4_bmff::BoxType;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::FourCC;

use crate::multiplex::AudioSampleDescription;
use crate::multiplex::SampleDescription;
use crate::multiplex::TrackId;
use crate::multiplex::VisualSampleDescription;

use super::Context;
use super::DataLayout;
use super::FragmentDefaults;

use super::Result;
use super::error::*;
use super::repr::*;

/// Parses the given `moov` box and constructs the corresponding `Context` and `Layout`.
pub fn parse_moov(moov: &MoovBoxView<'_>) -> Result<(Context, DataLayout, Vec<FragmentDefaults>)> {
    let mvhd = moov.mvhd()?;

    let mut context = Context::new(mvhd.timescale)?;
    let mut layout = DataLayout::new();

    for trak in moov.traks() {
        let trak = trak?;

        // tkhd
        let tkhd = trak.tkhd()?;
        let track_id = TrackId::new(tkhd.track_id).ok_or(
            Error::new(ErrorKind::InvalidFormat)
                .with_message(format!("Invalid track ID in tkhd box: {}", tkhd.track_id)),
        )?;

        // mdia → mdhd, hdlr, minf → stbl
        let mdia = trak.mdia()?;
        let mdhd = mdia.mdhd()?;

        let timescale = Timescale::new(mdhd.timescale).ok_or(
            Error::new(ErrorKind::InvalidFormat).with_message(format!(
                "Invalid timescale value in mdhd box: {}",
                mdhd.timescale
            )),
        )?;

        let hdlr = mdia.hdlr()?;
        let minf = mdia.minf()?;
        let stbl = minf.stbl()?;

        // stbl の子ボックスを一度だけ取得
        let stsd = stbl.stsd()?;

        // descriptions (stsd + hdlr)
        let descriptions = parse_sample_descriptions(hdlr.handler_type, &stsd)?;

        // edit list (edts/elst)
        let edit_list = match trak.edts()? {
            Some(edts) => parse_edit_list(&edts)?,
            None => None,
        };

        // samples (stts + ctts + stss)
        let samples = parse_samples(&stbl)?;

        extend_layout(&mut layout, track_id, &stbl)?;

        context.movie.tracks.push(Track {
            id: track_id,
            timescale,
            language: mdhd.language,
            matrix: tkhd.matrix,
            alternate_group: tkhd.alternate_group,
            descriptions,
            samples,
            edit_list,
        });
    }

    let defaults = if let Some(mvex) = moov.mvex()? {
        mvex.trexs()
            .map(|trex| parse_sample_defaults(&trex?))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    Ok((context, layout, defaults))
}

pub fn parse_moof(
    moof: &MoofBoxView<'_>,
    moof_offset: u64,
    defaults: &[FragmentDefaults],
) -> Result<(Vec<Sample>, DataLayout)> {
    todo!()
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

fn parse_samples(stbl: &StblBoxView<'_>) -> Result<Vec<Sample>> {
    let stts = stbl.stts()?;
    let ctts = stbl.ctts()?;
    let stss = stbl.stss()?;

    let all_sync = stss.is_none();
    let mut sync_iter = stss.into_iter().flat_map(|s| s.entries()).peekable();
    let mut ctts_entries = ctts.as_ref().map(|ctts| ctts.composition_time_offsets());

    let mut samples = Vec::new();
    for (i, delta) in stts.sample_deltas().enumerate() {
        let sample_number = i as u32 + 1;
        let cto = ctts_entries
            .as_mut()
            .map(|e| e.next().unwrap_or(0))
            .unwrap_or(0);

        // stss は昇順なので、カーソルを進めるだけで判定できる
        let is_sync = if all_sync {
            true
        } else {
            if sync_iter.peek().map(|e| e.sample_number) == Some(sample_number) {
                sync_iter.next();
                true
            } else {
                false
            }
        };

        samples.push(Sample {
            delta,
            is_sync,
            composition_time_offset: i64::from(cto),
        });
    }

    Ok(samples)
}

fn parse_edit_list(edts: &EdtsBoxView<'_>) -> Result<Option<Vec<Edit>>> {
    let Some(elst) = edts.elst()? else {
        return Ok(None);
    };

    let edits = elst
        .entries()?
        .map(|e| Edit {
            segment_duration: e.segment_duration,
            media_time: e.media_time,
            media_rate: e.media_rate,
        })
        .collect();

    Ok(Some(edits))
}

fn parse_chunk_offsets(stbl: &StblBoxView<'_>) -> Result<Vec<u64>> {
    if let Some(stco) = stbl.stco()? {
        Ok(stco.entries().map(|e| u64::from(e.chunk_offset)).collect())
    } else if let Some(co64) = stbl.co64()? {
        Ok(co64.entries().map(|e| e.chunk_offset).collect())
    } else {
        Err(Error::new(ErrorKind::InvalidFormat).with_message("Missing stco or co64 box"))
    }
}

fn parse_sample_sizes(stbl: &StblBoxView<'_>) -> Result<Vec<u32>> {
    if let Some(stsz) = stbl.stsz()? {
        if stsz.sample_size != 0 {
            Ok(alloc::vec![stsz.sample_size; stsz.sample_count as usize])
        } else {
            Ok(stsz.entries().map(|e| e.entry_size).collect())
        }
    } else if let Some(stz2) = stbl.stz2()? {
        Ok(stz2.entries().map(|e| u32::from(e.entry_size)).collect())
    } else {
        Err(Error::new(ErrorKind::InvalidFormat).with_message("Missing stsz or stz2 box"))
    }
}

fn parse_samples_per_chunk(stbl: &StblBoxView<'_>, chunk_count: usize) -> Result<Vec<u32>> {
    let stsc = stbl.stsc()?;
    Ok(stsc.chunk_sample_counts(chunk_count).collect())
}

fn parse_sample_defaults(trex: &TrexBox) -> Result<FragmentDefaults> {
    let track_id = TrackId::new(trex.track_id).ok_or(
        Error::new(ErrorKind::InvalidFormat)
            .with_message(format!("Invalid track ID in trex box: {}", trex.track_id)),
    )?;

    Ok(FragmentDefaults::new(track_id)
        .with_sample_description_index(trex.default_sample_description_index)
        .with_sample_duration(trex.default_sample_duration)
        .with_sample_size(trex.default_sample_size)
        .with_sample_flags(trex.default_sample_flags))
}

fn extend_layout(layout: &mut DataLayout, track_id: TrackId, stbl: &StblBoxView<'_>) -> Result<()> {
    // layout (stsc + stco/co64 + stsz/stz2)
    let chunk_offsets = parse_chunk_offsets(stbl)?;
    let sample_sizes = parse_sample_sizes(stbl)?;
    let samples_per_chunk = parse_samples_per_chunk(stbl, chunk_offsets.len())?;

    let mut cursor = 0usize;
    for (&offset, count) in chunk_offsets.iter().zip(samples_per_chunk) {
        let count = count as usize;
        let sizes = sample_sizes[cursor..cursor + count].to_vec();
        layout.add_chunk(track_id, offset, sizes)?;
        cursor += count;
    }

    Ok(())
}
