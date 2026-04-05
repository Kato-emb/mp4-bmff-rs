//!

use std::time::Duration;

use mp4_bmff::boxes::avc::Avc1SampleEntry;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::*;

use mp4_bmff_util::multiplex::mux::Muxer;
use mp4_bmff_util::multiplex::{MediaDefinition, Sample, VisualSampleDescription};

const SAMPLE_MP4: &[u8] = include_bytes!("samples/sample.mp4");

fn ticks_to_nanos(ticks: u64, timescale: u64) -> u64 {
    let secs = ticks / timescale;
    let remainder = ticks % timescale;
    secs * 1_000_000_000 + remainder * 1_000_000_000 / timescale
}

fn signed_ticks_to_nanos(ticks: i64, timescale: u64) -> i64 {
    let abs = ticks.unsigned_abs();
    let nanos = ticks_to_nanos(abs, timescale);
    if ticks < 0 {
        -(nanos as i64)
    } else {
        nanos as i64
    }
}

/// Parsed per-sample information extracted from the original MP4's sample table.
struct SampleInfo {
    dts_ns: u64,
    pts_ns: Option<i64>,
    duration: Duration,
    is_sync: bool,
    size: u32,
    chunk_idx: usize,
}

/// Per-chunk information extracted from the original MP4's sample table.
struct ChunkInfo {
    data_offset: u64,
}

/// Extracts per-sample and per-chunk information from a decoded StblBox.
fn extract_samples(stbl: &StblBox, timescale: u32) -> (Vec<ChunkInfo>, Vec<SampleInfo>) {
    // Expand stts run-length into per-sample durations
    let durations: Vec<u32> = stbl
        .stts
        .entries
        .iter()
        .flat_map(|e| std::iter::repeat_n(e.sample_delta, e.sample_count as usize))
        .collect();

    // Expand stsz into per-sample sizes
    let sizes: Vec<u32> = match &stbl.sample_size {
        SampleSize::Stsz(stsz) => {
            if stsz.sample_size != 0 {
                vec![stsz.sample_size; stsz.sample_count as usize]
            } else {
                stsz.entries.iter().map(|e| e.entry_size).collect()
            }
        }
        SampleSize::Stz2(_) => unimplemented!("stz2 not supported in test"),
    };

    let sample_count = sizes.len();

    // Build sync sample set (1-based indices)
    let sync_samples: std::collections::HashSet<u32> = match &stbl.stss {
        Some(stss) => stss.entries.iter().map(|e| e.sample_number).collect(),
        None => (1..=sample_count as u32).collect(), // all sync if no stss
    };

    // Expand ctts run-length into per-sample composition time offsets
    let cto: Vec<i32> = stbl
        .ctts
        .as_ref()
        .map(|ctts| {
            ctts.entries
                .iter()
                .flat_map(|e| std::iter::repeat_n(e.sample_offset, e.sample_count as usize))
                .collect()
        })
        .unwrap_or_default();

    // Resolve chunk offsets
    let chunk_offsets: Vec<u64> = match &stbl.chunk_offset {
        ChunkOffset::Stco(stco) => stco
            .entries
            .iter()
            .map(|e| u64::from(e.chunk_offset))
            .collect(),
        ChunkOffset::Co64(co64) => co64.entries.iter().map(|e| e.chunk_offset).collect(),
    };

    // Expand stsc into per-sample chunk assignments
    let num_chunks = chunk_offsets.len();
    let mut sample_chunk_map: Vec<usize> = Vec::with_capacity(sample_count);
    for (i, entry) in stbl.stsc.entries.iter().enumerate() {
        let next_first_chunk = if i + 1 < stbl.stsc.entries.len() {
            stbl.stsc.entries[i + 1].first_chunk
        } else {
            num_chunks as u32 + 1
        };
        for chunk_idx in entry.first_chunk..next_first_chunk {
            for _ in 0..entry.samples_per_chunk {
                sample_chunk_map.push((chunk_idx - 1) as usize); // 0-based
            }
        }
    }

    // Build chunk info
    let chunks: Vec<ChunkInfo> = chunk_offsets
        .iter()
        .map(|&data_offset| ChunkInfo { data_offset })
        .collect();

    // Compute DTS and PTS for each sample using cumulative ticks
    // to avoid rounding error accumulation (telescoping sum).
    let timescale_u64 = u64::from(timescale);
    let mut dts_ticks: u64 = 0;
    let mut samples = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
        let duration_ticks = u64::from(durations[i]);
        let dts_ns = ticks_to_nanos(dts_ticks, timescale_u64);
        let next_dts_ns = ticks_to_nanos(dts_ticks + duration_ticks, timescale_u64);
        let duration = Duration::from_nanos(next_dts_ns - dts_ns);

        let pts_ns = if !cto.is_empty() {
            let cto_ticks = i64::from(cto[i]);
            let pts_ticks = dts_ticks as i64 + cto_ticks;
            Some(signed_ticks_to_nanos(pts_ticks, timescale_u64))
        } else {
            None
        };

        let is_sync = sync_samples.contains(&(i as u32 + 1));

        samples.push(SampleInfo {
            dts_ns,
            pts_ns,
            duration,
            is_sync,
            size: sizes[i],
            chunk_idx: sample_chunk_map[i],
        });

        dts_ticks += duration_ticks;
    }

    (chunks, samples)
}

/// Finds and decodes the moov box from raw MP4 data.
fn decode_moov(data: &[u8]) -> MoovBox {
    for raw in iter_boxes(data) {
        let raw = raw.unwrap();
        if raw.boxtype() == BoxType::MOOV {
            return MoovBox::decode(raw.into_payload()).unwrap();
        }
    }
    panic!("No moov box found");
}

#[test]
fn mux_roundtrip_video() {
    // 1. Parse original MP4
    let original_moov = decode_moov(SAMPLE_MP4);
    let original_trak = &original_moov.traks[0];
    let original_mdhd = &original_trak.mdia.mdhd;
    let timescale = original_mdhd.timescale;

    // 2. Extract avc1 sample entry for MediaDefinition
    let stsd_entry = &original_trak.mdia.minf.stbl.stsd.entries[0];
    assert_eq!(stsd_entry.boxtype(), BoxType::AVC1);
    let avc1 = Avc1SampleEntry::decode(stsd_entry.payload()).unwrap();
    let media = MediaDefinition::Video {
        width: avc1.base.width,
        height: avc1.base.height,
        codec: VisualSampleDescription::Avc1(avc1.avcc.avc_config),
    };

    // 3. Extract sample info from original
    let (chunks, samples) = extract_samples(&original_trak.mdia.minf.stbl, timescale);
    assert_eq!(samples.len(), 3, "Expected 3 frames in sample.mp4");

    // 4. Re-mux using Muxer API
    let mut builder = Muxer::builder(original_moov.mvhd.timescale).unwrap();
    let track_id = builder.add_track(timescale, media).unwrap().build();
    let mut muxer = builder.build().unwrap();

    let mut current_chunk: Option<usize> = None;
    for s in &samples {
        if current_chunk != Some(s.chunk_idx) {
            muxer
                .begin_chunk(track_id, chunks[s.chunk_idx].data_offset)
                .unwrap();
            current_chunk = Some(s.chunk_idx);
        }
        muxer
            .add_sample(
                track_id,
                Sample::new(s.dts_ns, s.pts_ns, s.duration, s.is_sync, s.size),
            )
            .unwrap();
    }

    let result_moov = muxer.finalize().unwrap();

    // 5. Compare structural properties
    // mvhd
    assert_eq!(result_moov.mvhd.timescale, original_moov.mvhd.timescale);
    assert_eq!(result_moov.mvhd.duration, original_moov.mvhd.duration);
    assert_eq!(
        result_moov.mvhd.next_track_id,
        original_moov.mvhd.next_track_id
    );

    // trak count
    assert_eq!(result_moov.traks.len(), 1);
    let result_trak = &result_moov.traks[0];

    // tkhd
    assert_eq!(result_trak.tkhd.track_id, original_trak.tkhd.track_id);
    assert_eq!(result_trak.tkhd.duration, original_trak.tkhd.duration);
    assert_eq!(result_trak.tkhd.width, original_trak.tkhd.width);
    assert_eq!(result_trak.tkhd.height, original_trak.tkhd.height);

    // mdhd
    let result_mdhd = &result_trak.mdia.mdhd;
    assert_eq!(result_mdhd.timescale, original_mdhd.timescale);
    assert_eq!(result_mdhd.duration, original_mdhd.duration);

    // stbl - stts
    let result_stbl = &result_trak.mdia.minf.stbl;
    let original_stbl = &original_trak.mdia.minf.stbl;
    // stts: individual deltas may differ by +-1 tick due to nanosecond quantization,
    // but the total sample count and total duration must match exactly.
    let result_stts_total: u64 = result_stbl
        .stts
        .entries
        .iter()
        .map(|e| u64::from(e.sample_count) * u64::from(e.sample_delta))
        .sum();
    let original_stts_total: u64 = original_stbl
        .stts
        .entries
        .iter()
        .map(|e| u64::from(e.sample_count) * u64::from(e.sample_delta))
        .sum();
    assert_eq!(result_stts_total, original_stts_total);

    let result_sample_count: u32 = result_stbl
        .stts
        .entries
        .iter()
        .map(|e| e.sample_count)
        .sum();
    let original_sample_count: u32 = original_stbl
        .stts
        .entries
        .iter()
        .map(|e| e.sample_count)
        .sum();
    assert_eq!(result_sample_count, original_sample_count);

    // stbl - sample sizes
    match (&result_stbl.sample_size, &original_stbl.sample_size) {
        (SampleSize::Stsz(r), SampleSize::Stsz(o)) => {
            assert_eq!(r.sample_count, o.sample_count);
            if o.sample_size != 0 {
                // All same size: check either uniform or per-entry equivalent
                let r_sizes: Vec<u32> = if r.sample_size != 0 {
                    vec![r.sample_size; r.sample_count as usize]
                } else {
                    r.entries.iter().map(|e| e.entry_size).collect()
                };
                assert!(r_sizes.iter().all(|&s| s == o.sample_size));
            } else {
                assert_eq!(
                    r.entries.iter().map(|e| e.entry_size).collect::<Vec<_>>(),
                    o.entries.iter().map(|e| e.entry_size).collect::<Vec<_>>(),
                );
            }
        }
        _ => panic!("Expected both to be Stsz"),
    }

    // stbl - chunk offsets match
    assert_eq!(
        result_stbl.chunk_offset.entries_count(),
        original_stbl.chunk_offset.entries_count(),
    );

    // stbl - stss (sync samples) match
    assert_eq!(result_stbl.stss.is_some(), original_stbl.stss.is_some());
    if let (Some(r_stss), Some(o_stss)) = (&result_stbl.stss, &original_stbl.stss) {
        assert_eq!(
            r_stss
                .entries
                .iter()
                .map(|e| e.sample_number)
                .collect::<Vec<_>>(),
            o_stss
                .entries
                .iter()
                .map(|e| e.sample_number)
                .collect::<Vec<_>>(),
        );
    }
}

#[test]
fn mux_empty_track_rejected() {
    let media = MediaDefinition::Other(mp4_bmff::types::FourCC::new(*b"test"));
    let mut builder = Muxer::builder(1000).unwrap();
    builder.add_track(1000, media).unwrap().build();
    let muxer = builder.build().unwrap();

    let result = muxer.finalize();
    assert!(result.is_err());
}

#[test]
fn mux_multiple_tracks() {
    // Use the avc1 entry from sample.mp4 for both tracks
    let original_moov = decode_moov(SAMPLE_MP4);
    let stsd_entry = &original_moov.traks[0].mdia.minf.stbl.stsd.entries[0];
    let avc1_1 = Avc1SampleEntry::decode(stsd_entry.payload()).unwrap();
    let avc1_2 = Avc1SampleEntry::decode(stsd_entry.payload()).unwrap();

    let media1 = MediaDefinition::Video {
        width: avc1_1.base.width,
        height: avc1_1.base.height,
        codec: VisualSampleDescription::Avc1(avc1_1.avcc.avc_config),
    };
    let media2 = MediaDefinition::Video {
        width: avc1_2.base.width,
        height: avc1_2.base.height,
        codec: VisualSampleDescription::Avc1(avc1_2.avcc.avc_config),
    };

    let mut builder = Muxer::builder(1000).unwrap();
    let tid1 = builder.add_track(90000, media1).unwrap().build();
    let tid2 = builder.add_track(90000, media2).unwrap().build();
    let mut muxer = builder.build().unwrap();

    muxer.begin_chunk(tid1, 0).unwrap();
    muxer
        .add_sample(
            tid1,
            Sample::new(0, None, Duration::from_millis(33), true, 1000),
        )
        .unwrap();
    muxer
        .add_sample(
            tid1,
            Sample::new(33_000_000, None, Duration::from_millis(33), false, 800),
        )
        .unwrap();
    muxer.begin_chunk(tid2, 2000).unwrap();
    muxer
        .add_sample(
            tid2,
            Sample::new(0, None, Duration::from_millis(33), true, 500),
        )
        .unwrap();
    muxer
        .add_sample(
            tid2,
            Sample::new(33_000_000, None, Duration::from_millis(33), true, 500),
        )
        .unwrap();

    let moov = muxer.finalize().unwrap();

    assert_eq!(moov.traks.len(), 2);
    assert_eq!(moov.mvhd.next_track_id, 3);
    assert_eq!(moov.traks[0].tkhd.track_id, 1);
    assert_eq!(moov.traks[1].tkhd.track_id, 2);
}
