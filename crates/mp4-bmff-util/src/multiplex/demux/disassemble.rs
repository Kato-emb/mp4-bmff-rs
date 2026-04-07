use core::num::NonZeroU32;
use core::time::Duration;

use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::BoxDecode;
use mp4_bmff::types::FourCC;
use mp4_bmff::{BoxType, RawBoxOwned};

use crate::multiplex::error::{Error, ErrorKind};
use crate::multiplex::{
    AudioSampleDescription, Chunk, EditSegment, MediaDefinition, Result, Sample,
    VisualSampleDescription,
};

// ---------------------------------------------------------------------------
// TickScale — timescale-bound converter (ticks → nanos)
// ---------------------------------------------------------------------------

/// Timescale-bound converter for tick → nanosecond transformations.
///
/// This is the reverse of the mux-side `TickScale` which converts nanos → ticks.
/// The formula mirrors `tests/mux.rs::ticks_to_nanos`.
struct TickScale(NonZeroU32);

impl TickScale {
    /// Converts ticks to nanoseconds.
    fn to_nanos(&self, ticks: u64) -> u64 {
        let ts = u64::from(self.0.get());
        let secs = ticks / ts;
        let remainder = ticks % ts;
        secs * 1_000_000_000 + remainder * 1_000_000_000 / ts
    }

    /// Converts signed ticks to signed nanoseconds.
    fn signed_to_nanos(&self, ticks: i64) -> i64 {
        let abs = self.to_nanos(ticks.unsigned_abs());
        if ticks < 0 { -(abs as i64) } else { abs as i64 }
    }

    /// Converts ticks to a `Duration`.
    fn to_duration(&self, ticks: u64) -> Duration {
        Duration::from_nanos(self.to_nanos(ticks))
    }
}

fn sample_duration(
    decode_time_ticks: u64,
    delta_ticks: u32,
    scale: &TickScale,
) -> Result<Duration> {
    let start = decode_time_ticks;
    let end = start
        .checked_add(u64::from(delta_ticks))
        .ok_or(Error::new(ErrorKind::Overflow))?;
    Ok(scale.to_duration(end) - scale.to_duration(start))
}

fn sample_pts_ns(decode_time_ticks: u64, cto_ticks: i32, scale: &TickScale) -> Result<Option<i64>> {
    if cto_ticks == 0 {
        return Ok(None);
    }

    let pts_ticks = i64::try_from(decode_time_ticks)
        .ok()
        .and_then(|dt| dt.checked_add(i64::from(cto_ticks)))
        .ok_or(Error::new(ErrorKind::Overflow))?;
    Ok(Some(scale.signed_to_nanos(pts_ticks)))
}

fn parse_media_definition(stsd: &StsdBox, hdlr: &HdlrBox) -> Result<MediaDefinition> {
    let entry = stsd.entries.first().ok_or(
        Error::new(ErrorKind::InvalidInput).with_message("stsd must contain at least one entry"),
    )?;

    let vide = FourCC::new(*b"vide");
    let soun = FourCC::new(*b"soun");

    if hdlr.handler_type == vide {
        parse_visual_media(entry)
    } else if hdlr.handler_type == soun {
        parse_audio_media(entry)
    } else {
        Ok(MediaDefinition::Other(hdlr.handler_type))
    }
}

fn parse_visual_media(entry: &RawBoxOwned) -> Result<MediaDefinition> {
    match entry.boxtype() {
        #[cfg(feature = "avc")]
        BoxType::AVC1 => {
            use mp4_bmff::boxes::avc::Avc1SampleEntry;
            let avc1 = Avc1SampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
            Ok(MediaDefinition::Video {
                width: avc1.base.width,
                height: avc1.base.height,
                codec: VisualSampleDescription::Avc1(avc1.avcc.avc_config),
            })
        }
        #[cfg(feature = "avc")]
        BoxType::AVC3 => {
            use mp4_bmff::boxes::avc::Avc3SampleEntry;
            let avc3 = Avc3SampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
            Ok(MediaDefinition::Video {
                width: avc3.base.width,
                height: avc3.base.height,
                codec: VisualSampleDescription::Avc3(avc3.avcc.avc_config),
            })
        }
        #[cfg(feature = "mp4")]
        BoxType::MP4V => {
            use mp4_bmff::boxes::mp4::Mp4vSampleEntry;
            let mp4v = Mp4vSampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
            let dec_specific_info = mp4v
                .esds
                .esd
                .dec_config_descr
                .dec_specific_info
                .map(|d| d.instance().to_vec())
                .unwrap_or_default();
            Ok(MediaDefinition::Video {
                width: mp4v.base.width,
                height: mp4v.base.height,
                codec: VisualSampleDescription::Mp4v(dec_specific_info),
            })
        }
        _ => {
            Err(Error::new(ErrorKind::Unsupported)
                .with_message("Unsupported visual sample entry type"))
        }
    }
}

fn parse_audio_media(entry: &RawBoxOwned) -> Result<MediaDefinition> {
    match entry.boxtype() {
        #[cfg(feature = "mp4")]
        BoxType::MP4A => {
            use mp4_bmff::boxes::mp4::Mp4aSampleEntry;
            let mp4a = Mp4aSampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
            let dec_specific_info = mp4a
                .esds
                .esd
                .dec_config_descr
                .dec_specific_info
                .map(|d| d.instance().to_vec())
                .unwrap_or_default();
            Ok(MediaDefinition::Audio {
                channel_count: mp4a.base.channelcount,
                sample_rate: mp4a.base.samplerate.integer() as u16,
                codec: AudioSampleDescription::Mp4a(dec_specific_info),
            })
        }
        _ => {
            Err(Error::new(ErrorKind::Unsupported)
                .with_message("Unsupported audio sample entry type"))
        }
    }
}

// ---------------------------------------------------------------------------
// Non-fragmented: stbl disassembly
// ---------------------------------------------------------------------------

fn parse_sample_table(stbl: &StblBox, scale: &TickScale) -> Result<Vec<Chunk>> {
    let num_chunks = stbl.chunk_offset.entries_count();

    let mut sample_deltas = stbl.stts.sample_deltas();

    let mut sample_ctos = stbl
        .ctts
        .as_ref()
        .map(|ctts| ctts.composition_time_offsets());

    let mut chunks = Vec::with_capacity(num_chunks);
    let mut decode_time_ticks: u64 = 0;
    let mut sample_number: u32 = 0;

    for (data_offset, sample_count) in stbl
        .chunk_offset
        .iter()
        .zip(stbl.stsc.chunk_sample_counts(num_chunks))
    {
        let mut samples: Vec<Sample> = Vec::with_capacity(sample_count as usize);

        for _ in 0..sample_count {
            sample_number = sample_number.checked_add(1).ok_or(ErrorKind::Overflow)?;

            let delta_ticks = sample_deltas
                .next()
                .ok_or(Error::new(ErrorKind::InvalidInput))?;
            let dts_ns = scale.to_nanos(decode_time_ticks);

            let pts_ns = if let Some(sample_ctos) = sample_ctos.as_mut() {
                let cto_ticks = sample_ctos
                    .next()
                    .ok_or(Error::new(ErrorKind::InvalidInput))?;
                sample_pts_ns(decode_time_ticks, cto_ticks, scale)?
            } else {
                None
            };

            let duration = sample_duration(decode_time_ticks, delta_ticks, scale)?;

            let is_sync = stbl
                .stss
                .as_ref()
                .map_or(true, |stss| stss.contains(sample_number));

            let size = stbl
                .sample_size
                .get(sample_number as usize - 1)
                .ok_or(Error::new(ErrorKind::InvalidInput))?;

            let sample = Sample::new(dts_ns, pts_ns, duration, is_sync, size)?;
            samples.push(sample);

            decode_time_ticks = decode_time_ticks
                .checked_add(u64::from(delta_ticks))
                .ok_or(ErrorKind::Overflow)?;
        }

        chunks.push(Chunk {
            data_offset,
            samples,
        });
    }

    Ok(chunks)
}

// ---------------------------------------------------------------------------
// edts → EditSegment
// ---------------------------------------------------------------------------

/// Parses an optional `EdtsBox` into an optional list of `EditSegment`s.
fn parse_edit_list(
    edts: Option<&EdtsBox>,
    movie_timescale: NonZeroU32,
    media_timescale: NonZeroU32,
) -> Result<Option<Vec<EditSegment>>> {
    let Some(edts) = edts else { return Ok(None) };
    let Some(elst) = &edts.elst else {
        return Ok(None);
    };

    let movie_scale = TickScale(movie_timescale);
    let media_scale = TickScale(media_timescale);

    let mut segments = Vec::with_capacity(elst.entries.len());

    for entry in &elst.entries {
        let duration = movie_scale.to_duration(entry.segment_duration);

        let segment = if entry.media_time == -1 {
            EditSegment::Empty { duration }
        } else if entry.media_rate.to_f32() == 0.0 {
            let media_time = media_scale.to_duration(entry.media_time as u64);
            EditSegment::Dwell {
                duration,
                media_time,
            }
        } else {
            let media_start = media_scale.to_duration(entry.media_time as u64);
            EditSegment::Media {
                duration,
                media_start,
                media_rate: entry.media_rate.to_f32(),
            }
        };

        segments.push(segment);
    }

    Ok(Some(segments))
}
