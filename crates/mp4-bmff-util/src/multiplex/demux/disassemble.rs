use core::num::NonZeroU32;
use core::time::Duration;

use alloc::collections::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::BoxDecode;
use mp4_bmff::types::FourCC;
use mp4_bmff::{BoxType, RawBoxOwned};

use crate::multiplex::error::{Error, ErrorKind};
use crate::multiplex::{
    AudioSampleDescription, Chunk, EditSegment, MediaDefinition, Result, Sample, Track, TrackId,
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
        if ticks < 0 {
            -(abs as i64)
        } else {
            abs as i64
        }
    }

    /// Converts ticks to a `Duration`.
    fn to_duration(&self, ticks: u64) -> Duration {
        Duration::from_nanos(self.to_nanos(ticks))
    }
}

// ---------------------------------------------------------------------------
// Non-fragmented: stbl disassembly
// ---------------------------------------------------------------------------

/// Disassembles a `StblBox` into a list of `Chunk`s.
///
/// This is the reverse of `mux/assemble.rs::build_stbl`. It expands
/// run-length encoded tables (stts, stsc, ctts) and resolves chunk offsets
/// to produce `Chunk` values containing `Sample`s with nanosecond timestamps.
pub(super) fn disassemble_stbl(stbl: &StblBox, timescale: NonZeroU32) -> Result<Vec<Chunk>> {
    let scale = TickScale(timescale);

    // 1. Expand stts run-length into per-sample durations (ticks)
    let durations: Vec<u32> = stbl
        .stts
        .entries
        .iter()
        .flat_map(|e| core::iter::repeat_n(e.sample_delta, e.sample_count as usize))
        .collect();

    // 2. Expand sample sizes
    let sizes: Vec<u32> = expand_sample_sizes(&stbl.sample_size)?;
    let sample_count = sizes.len();

    // Validate consistency between stts and stsz
    if durations.len() != sample_count {
        return Err(Error::new(ErrorKind::InvalidInput).with_message(
            "Sample count mismatch between stts and stsz",
        ));
    }

    // 3. Build sync sample set (1-based indices)
    let sync_samples: BTreeSet<u32> = match &stbl.stss {
        Some(stss) => stss.entries.iter().map(|e| e.sample_number).collect(),
        None => (1..=sample_count as u32).collect(),
    };

    // 4. Expand ctts run-length into per-sample composition time offsets
    let cto: Vec<i32> = stbl
        .ctts
        .as_ref()
        .map(|ctts| {
            ctts.entries
                .iter()
                .flat_map(|e| core::iter::repeat_n(e.sample_offset, e.sample_count as usize))
                .collect()
        })
        .unwrap_or_default();

    // 5. Resolve chunk offsets
    let chunk_offsets = expand_chunk_offsets(&stbl.chunk_offset);
    let num_chunks = chunk_offsets.len();

    // 6. Expand stsc into per-sample chunk assignments (0-based)
    let sample_chunk_map = expand_stsc(&stbl.stsc, num_chunks, sample_count)?;

    // 7. Build samples with telescoping DTS/duration computation
    let mut dts_ticks: u64 = 0;
    let mut chunk_samples: Vec<Vec<Sample>> = vec![Vec::new(); num_chunks];

    for i in 0..sample_count {
        let delta_ticks = u64::from(durations[i]);
        let dts_ns = scale.to_nanos(dts_ticks);
        let next_dts_ns = scale.to_nanos(dts_ticks + delta_ticks);
        let duration = Duration::from_nanos(next_dts_ns - dts_ns);

        let pts_ns = if !cto.is_empty() {
            let cto_ticks = i64::from(cto[i]);
            let pts_ticks = dts_ticks as i64 + cto_ticks;
            Some(scale.signed_to_nanos(pts_ticks))
        } else {
            None
        };

        let is_sync = sync_samples.contains(&(i as u32 + 1));
        let sample = Sample::new(dts_ns, pts_ns, duration, is_sync, sizes[i])?;

        chunk_samples[sample_chunk_map[i]].push(sample);
        dts_ticks += delta_ticks;
    }

    // 8. Build Chunk values
    let chunks = chunk_offsets
        .into_iter()
        .zip(chunk_samples)
        .filter(|(_, samples)| !samples.is_empty())
        .map(|(offset, samples)| Chunk::with_samples(offset, samples))
        .collect::<Result<Vec<_>>>()?;

    Ok(chunks)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Expands `SampleSize` (stsz or stz2) into per-sample sizes.
fn expand_sample_sizes(sample_size: &SampleSize) -> Result<Vec<u32>> {
    match sample_size {
        SampleSize::Stsz(stsz) => {
            if stsz.sample_size != 0 {
                Ok(vec![stsz.sample_size; stsz.sample_count as usize])
            } else {
                Ok(stsz.entries.iter().map(|e| e.entry_size).collect())
            }
        }
        SampleSize::Stz2(stz2) => Ok(stz2
            .entries
            .iter()
            .map(|e| u32::from(e.entry_size))
            .collect()),
    }
}

/// Expands chunk offsets from `ChunkOffset` (stco or co64) into `Vec<u64>`.
fn expand_chunk_offsets(chunk_offset: &ChunkOffset) -> Vec<u64> {
    match chunk_offset {
        ChunkOffset::Stco(stco) => stco
            .entries
            .iter()
            .map(|e| u64::from(e.chunk_offset))
            .collect(),
        ChunkOffset::Co64(co64) => co64.entries.iter().map(|e| e.chunk_offset).collect(),
    }
}

/// Expands `StscBox` into per-sample chunk assignments (0-based chunk index).
fn expand_stsc(stsc: &StscBox, num_chunks: usize, sample_count: usize) -> Result<Vec<usize>> {
    let mut map: Vec<usize> = Vec::with_capacity(sample_count);

    for (i, entry) in stsc.entries.iter().enumerate() {
        let next_first_chunk = if i + 1 < stsc.entries.len() {
            stsc.entries[i + 1].first_chunk
        } else {
            num_chunks as u32 + 1
        };
        for chunk_idx in entry.first_chunk..next_first_chunk {
            for _ in 0..entry.samples_per_chunk {
                map.push((chunk_idx - 1) as usize);
            }
        }
    }

    if map.len() != sample_count {
        return Err(Error::new(ErrorKind::InvalidInput).with_message(
            "Sample count mismatch between stsc expansion and stsz",
        ));
    }

    Ok(map)
}

// ---------------------------------------------------------------------------
// Non-fragmented: trak disassembly
// ---------------------------------------------------------------------------

/// Disassembles a `TrakBox` into a `Track` and its `Chunk`s.
pub(super) fn disassemble_trak(
    trak: &TrakBox,
    movie_timescale: NonZeroU32,
) -> Result<(Track, Vec<Chunk>)> {
    let tkhd = &trak.tkhd;
    let mdia = &trak.mdia;
    let mdhd = &mdia.mdhd;

    // Validate track ID
    let track_id = TrackId::new(tkhd.track_id).ok_or(
        Error::new(ErrorKind::InvalidInput).with_message("Track ID must be non-zero"),
    )?;

    // Validate media timescale
    let media_timescale = NonZeroU32::new(mdhd.timescale).ok_or(
        Error::new(ErrorKind::InvalidInput).with_message("Media timescale must be non-zero"),
    )?;

    // Parse media definition from stsd + handler type
    let handler_type = mdia.hdlr.handler_type;
    let media = parse_media_definition(&mdia.minf.stbl.stsd, handler_type)?;

    // Parse edit list
    let edit_list = parse_edit_list(trak.edts.as_ref(), movie_timescale, media_timescale)?;

    // Build Track
    let mut track = Track::new(track_id, media_timescale, media);
    track.language = mdhd.language;
    track.matrix = tkhd.matrix;
    track.alternate_group = tkhd.alternate_group;
    track.edit_list = edit_list;

    // Disassemble sample table
    let chunks = disassemble_stbl(&mdia.minf.stbl, media_timescale)?;

    Ok((track, chunks))
}

// ---------------------------------------------------------------------------
// stsd → MediaDefinition
// ---------------------------------------------------------------------------

/// Parses the first sample description entry into a `MediaDefinition`.
fn parse_media_definition(stsd: &StsdBox, handler_type: FourCC) -> Result<MediaDefinition> {
    let entry = stsd.entries.first().ok_or(
        Error::new(ErrorKind::InvalidInput).with_message("stsd must contain at least one entry"),
    )?;

    let vide = FourCC::new(*b"vide");
    let soun = FourCC::new(*b"soun");

    if handler_type == vide {
        parse_visual_media(entry)
    } else if handler_type == soun {
        parse_audio_media(entry)
    } else {
        Ok(MediaDefinition::Other(handler_type))
    }
}

fn parse_visual_media(entry: &RawBoxOwned) -> Result<MediaDefinition> {
    match entry.boxtype() {
        #[cfg(feature = "avc")]
        BoxType::AVC1 => {
            use mp4_bmff::boxes::avc::Avc1SampleEntry;
            let avc1 =
                Avc1SampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
            Ok(MediaDefinition::Video {
                width: avc1.base.width,
                height: avc1.base.height,
                codec: VisualSampleDescription::Avc1(avc1.avcc.avc_config),
            })
        }
        #[cfg(feature = "avc")]
        BoxType::AVC3 => {
            use mp4_bmff::boxes::avc::Avc3SampleEntry;
            let avc3 =
                Avc3SampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
            Ok(MediaDefinition::Video {
                width: avc3.base.width,
                height: avc3.base.height,
                codec: VisualSampleDescription::Avc3(avc3.avcc.avc_config),
            })
        }
        #[cfg(feature = "mp4")]
        BoxType::MP4V => {
            use mp4_bmff::boxes::mp4::Mp4vSampleEntry;
            let mp4v =
                Mp4vSampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
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
        _ => Err(Error::new(ErrorKind::Unsupported)
            .with_message("Unsupported visual sample entry type")),
    }
}

fn parse_audio_media(entry: &RawBoxOwned) -> Result<MediaDefinition> {
    match entry.boxtype() {
        #[cfg(feature = "mp4")]
        BoxType::MP4A => {
            use mp4_bmff::boxes::mp4::Mp4aSampleEntry;
            let mp4a =
                Mp4aSampleEntry::decode(entry.payload()).map_err(Error::box_decode)?;
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
        _ => Err(Error::new(ErrorKind::Unsupported)
            .with_message("Unsupported audio sample entry type")),
    }
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

// ---------------------------------------------------------------------------
// Fragmented: traf disassembly
// ---------------------------------------------------------------------------

use super::types::TrackDefaults;

/// Disassembles a `TrafBox` into a list of `Chunk`s.
///
/// Default values cascade: trex defaults → tfhd defaults → per-sample trun entry.
/// Each `TrunBox` produces one `Chunk`.
pub(super) fn disassemble_traf(
    traf: &TrafBox,
    timescale: NonZeroU32,
    trex_defaults: &TrackDefaults,
) -> Result<Vec<Chunk>> {
    let scale = TickScale(timescale);
    let tfhd = &traf.tfhd;

    // Resolve default values: tfhd overrides trex
    let default_duration = tfhd
        .default_sample_duration
        .unwrap_or(trex_defaults.default_sample_duration);
    let default_size = tfhd
        .default_sample_size
        .unwrap_or(trex_defaults.default_sample_size);
    let default_flags = tfhd
        .default_sample_flags
        .unwrap_or(trex_defaults.default_sample_flags);

    // Base decode time from tfdt
    let base_decode_time = traf.tfdt.as_ref().map_or(0u64, |tfdt| tfdt.base_media_decode_time);

    let mut chunks = Vec::with_capacity(traf.truns.len());
    let mut dts_ticks = base_decode_time;

    for trun in &traf.truns {
        let data_offset = trun.data_offset().unwrap_or(0) as u64;
        let first_sample_flags = trun.first_sample_flags();

        let mut samples = Vec::with_capacity(trun.entries().len());

        for (i, entry) in trun.entries().iter().enumerate() {
            let duration_ticks = entry.duration.unwrap_or(default_duration);
            let size = entry.size.unwrap_or(default_size);

            // Flags cascade: first_sample_flags (for index 0) > per-entry > default
            let flags = if i == 0 {
                first_sample_flags.or(entry.flags).unwrap_or(default_flags)
            } else {
                entry.flags.unwrap_or(default_flags)
            };
            let is_sync = flags.is_sync();

            let dts_ns = scale.to_nanos(dts_ticks);
            let next_dts_ns = scale.to_nanos(dts_ticks + u64::from(duration_ticks));
            let duration = Duration::from_nanos(next_dts_ns - dts_ns);

            let pts_ns = entry.composition_time_offset.map(|cto| {
                let pts_ticks = dts_ticks as i64 + cto;
                scale.signed_to_nanos(pts_ticks)
            });

            let sample = Sample::new(dts_ns, pts_ns, duration, is_sync, size)?;
            samples.push(sample);
            dts_ticks += u64::from(duration_ticks);
        }

        if !samples.is_empty() {
            chunks.push(Chunk::with_samples(data_offset, samples)?);
        }
    }

    Ok(chunks)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(v: u32) -> NonZeroU32 {
        NonZeroU32::new(v).unwrap()
    }

    // --- TickScale ---

    #[test]
    fn to_nanos_zero() {
        assert_eq!(TickScale(nz(90000)).to_nanos(0), 0);
    }

    #[test]
    fn to_nanos_one_second() {
        assert_eq!(TickScale(nz(90000)).to_nanos(90000), 1_000_000_000);
    }

    #[test]
    fn to_nanos_subsecond() {
        assert_eq!(TickScale(nz(1000)).to_nanos(500), 500_000_000);
    }

    #[test]
    fn to_nanos_fractional() {
        // 2970 ticks at 90000 Hz = 33ms
        assert_eq!(TickScale(nz(90000)).to_nanos(2970), 33_000_000);
    }

    #[test]
    fn signed_to_nanos_positive() {
        assert_eq!(TickScale(nz(90000)).signed_to_nanos(90000), 1_000_000_000);
    }

    #[test]
    fn signed_to_nanos_negative() {
        assert_eq!(
            TickScale(nz(90000)).signed_to_nanos(-90000),
            -1_000_000_000
        );
    }

    #[test]
    fn to_duration_one_second() {
        assert_eq!(
            TickScale(nz(44100)).to_duration(44100),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn to_duration_fractional() {
        assert_eq!(
            TickScale(nz(90000)).to_duration(2970),
            Duration::from_millis(33)
        );
    }

    // --- disassemble_stbl ---

    /// Helper to build a minimal StblBox for testing.
    fn make_stbl(
        stts_entries: Vec<SttsEntry>,
        sizes: Vec<u32>,
        stsc_entries: Vec<StscEntry>,
        chunk_offsets: Vec<u64>,
        ctts: Option<Vec<CttsEntry>>,
        stss: Option<Vec<u32>>,
    ) -> StblBox {
        let mut stts = SttsBox::default();
        stts.entries = stts_entries;

        let mut stsz = StszBox::default();
        for &size in &sizes {
            stsz.push(size);
        }

        let mut stsc = StscBox::default();
        stsc.entries = stsc_entries;

        let mut stco = StcoBox::default();
        for &offset in &chunk_offsets {
            stco.entries.push(StcoEntry {
                chunk_offset: offset as u32,
            });
        }

        let ctts = ctts.map(|entries| {
            let mut ctts = CttsBox::default();
            ctts.entries = entries;
            ctts
        });

        let stss = stss.map(|numbers| {
            let mut stss = StssBox::default();
            for n in numbers {
                stss.push(n);
            }
            stss
        });

        StblBox {
            stsd: StsdBox::default(),
            stts,
            sample_size: SampleSize::Stsz(stsz),
            stsc,
            chunk_offset: ChunkOffset::Stco(stco),
            ctts,
            stss,
            stdp: None,
            cslg: None,
            stsh: None,
            sdtp: None,
            sbgps: Vec::new(),
            sgpds: Vec::new(),
        }
    }

    #[test]
    fn disassemble_stbl_single_chunk_single_sample() {
        let stbl = make_stbl(
            vec![SttsEntry { sample_count: 1, sample_delta: 90000 }],
            vec![1000],
            vec![StscEntry { first_chunk: 1, samples_per_chunk: 1, sample_description_index: 1 }],
            vec![0],
            None,
            None,
        );

        let chunks = disassemble_stbl(&stbl, nz(90000)).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data_offset(), 0);
        assert_eq!(chunks[0].sample_count(), 1);

        let sample = chunks[0].first_sample();
        assert_eq!(sample.dts_ns(), 0);
        assert_eq!(sample.pts_ns(), None);
        assert_eq!(sample.duration(), Duration::from_secs(1));
        assert!(sample.is_sync());
        assert_eq!(sample.size(), 1000);
    }

    #[test]
    fn disassemble_stbl_single_chunk_multiple_samples() {
        let stbl = make_stbl(
            vec![SttsEntry { sample_count: 3, sample_delta: 2970 }],
            vec![1000, 800, 900],
            vec![StscEntry { first_chunk: 1, samples_per_chunk: 3, sample_description_index: 1 }],
            vec![100],
            None,
            None,
        );

        let chunks = disassemble_stbl(&stbl, nz(90000)).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data_offset(), 100);
        assert_eq!(chunks[0].sample_count(), 3);

        let samples: Vec<_> = chunks[0].samples().collect();
        assert_eq!(samples[0].dts_ns(), 0);
        assert_eq!(samples[1].dts_ns(), 33_000_000);
        assert_eq!(samples[2].dts_ns(), 66_000_000);
        assert_eq!(samples[0].size(), 1000);
        assert_eq!(samples[1].size(), 800);
        assert_eq!(samples[2].size(), 900);
    }

    #[test]
    fn disassemble_stbl_multiple_chunks() {
        let stbl = make_stbl(
            vec![SttsEntry { sample_count: 4, sample_delta: 1000 }],
            vec![100, 200, 300, 400],
            vec![
                StscEntry { first_chunk: 1, samples_per_chunk: 2, sample_description_index: 1 },
            ],
            vec![0, 5000],
            None,
            None,
        );

        let chunks = disassemble_stbl(&stbl, nz(1000)).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].data_offset(), 0);
        assert_eq!(chunks[0].sample_count(), 2);
        assert_eq!(chunks[1].data_offset(), 5000);
        assert_eq!(chunks[1].sample_count(), 2);
    }

    #[test]
    fn disassemble_stbl_with_ctts() {
        let stbl = make_stbl(
            vec![SttsEntry { sample_count: 2, sample_delta: 90000 }],
            vec![1000, 1000],
            vec![StscEntry { first_chunk: 1, samples_per_chunk: 2, sample_description_index: 1 }],
            vec![0],
            Some(vec![CttsEntry { sample_count: 2, sample_offset: 5940 }]),
            None,
        );

        let chunks = disassemble_stbl(&stbl, nz(90000)).unwrap();
        let samples: Vec<_> = chunks[0].samples().collect();
        // PTS = DTS + CTO: 0 + 5940 ticks = 66_000_000 ns
        assert_eq!(samples[0].pts_ns(), Some(66_000_000));
        assert_eq!(samples[1].pts_ns(), Some(1_066_000_000));
    }

    #[test]
    fn disassemble_stbl_with_stss() {
        let stbl = make_stbl(
            vec![SttsEntry { sample_count: 3, sample_delta: 1000 }],
            vec![100, 100, 100],
            vec![StscEntry { first_chunk: 1, samples_per_chunk: 3, sample_description_index: 1 }],
            vec![0],
            None,
            Some(vec![1]), // only first sample is sync
        );

        let chunks = disassemble_stbl(&stbl, nz(1000)).unwrap();
        let samples: Vec<_> = chunks[0].samples().collect();
        assert!(samples[0].is_sync());
        assert!(!samples[1].is_sync());
        assert!(!samples[2].is_sync());
    }

    #[test]
    fn disassemble_stbl_no_stss_all_sync() {
        let stbl = make_stbl(
            vec![SttsEntry { sample_count: 2, sample_delta: 1000 }],
            vec![100, 100],
            vec![StscEntry { first_chunk: 1, samples_per_chunk: 2, sample_description_index: 1 }],
            vec![0],
            None,
            None, // no stss = all sync
        );

        let chunks = disassemble_stbl(&stbl, nz(1000)).unwrap();
        for sample in chunks[0].samples() {
            assert!(sample.is_sync());
        }
    }

    #[test]
    fn disassemble_stbl_stts_stsz_mismatch() {
        let stbl = make_stbl(
            vec![SttsEntry { sample_count: 2, sample_delta: 1000 }],
            vec![100, 100, 100], // 3 sizes but 2 durations
            vec![StscEntry { first_chunk: 1, samples_per_chunk: 3, sample_description_index: 1 }],
            vec![0],
            None,
            None,
        );

        let result = disassemble_stbl(&stbl, nz(1000));
        assert!(result.is_err());
    }
}
