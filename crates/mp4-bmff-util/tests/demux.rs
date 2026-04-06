//!

use std::time::Duration;

use mp4_bmff::boxes::avc::Avc1SampleEntry;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::*;

use mp4_bmff_util::multiplex::demux::{Demuxer, FragmentedDemuxer};
use mp4_bmff_util::multiplex::mux::{FragmentedMuxer, Muxer};
use mp4_bmff_util::multiplex::{Chunk, MediaDefinition, Sample, VisualSampleDescription};

const SAMPLE_MP4: &[u8] = include_bytes!("samples/sample.mp4");

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
fn demux_real_file() {
    let moov = decode_moov(SAMPLE_MP4);
    let demuxer = Demuxer::new(&moov).unwrap();

    let tracks: Vec<_> = demuxer.tracks().collect();
    assert_eq!(tracks.len(), 1);

    let track = tracks[0];
    assert_eq!(track.id().get(), 1);

    let chunks = demuxer.chunks(track.id()).unwrap();
    assert!(!chunks.is_empty());

    // Verify total sample count matches the original
    let total_samples: usize = chunks.iter().map(|c| c.sample_count()).sum();
    assert_eq!(total_samples, 3, "Expected 3 frames in sample.mp4");

    // All samples should have positive size and duration
    for chunk in chunks {
        for sample in chunk.samples() {
            assert!(sample.size() > 0);
            assert!(sample.duration() > Duration::ZERO);
        }
    }
}

#[test]
fn demux_remux_roundtrip() {
    // 1. Demux the real file
    let original_moov = decode_moov(SAMPLE_MP4);
    let demuxer = Demuxer::new(&original_moov).unwrap();

    let tracks: Vec<_> = demuxer.tracks().collect();
    let track = tracks[0];
    let chunks = demuxer.chunks(track.id()).unwrap();

    // 2. Extract codec info from original for remuxing
    let stsd_entry = &original_moov.traks[0].mdia.minf.stbl.stsd.entries[0];
    let avc1 = Avc1SampleEntry::decode(stsd_entry.payload()).unwrap();
    let media = MediaDefinition::Video {
        width: avc1.base.width,
        height: avc1.base.height,
        codec: VisualSampleDescription::Avc1(avc1.avcc.avc_config),
    };

    // 3. Remux using Muxer API
    let mut builder = Muxer::builder(original_moov.mvhd.timescale).unwrap();
    let new_track_id = builder
        .add_track(original_moov.traks[0].mdia.mdhd.timescale, media)
        .unwrap()
        .build();
    let mut muxer = builder.build().unwrap();

    for chunk in chunks {
        muxer.add_chunk(new_track_id, chunk.clone()).unwrap();
    }

    let result_moov = muxer.finalize().unwrap();

    // 4. Compare structural properties
    assert_eq!(result_moov.mvhd.timescale, original_moov.mvhd.timescale);
    assert_eq!(result_moov.mvhd.duration, original_moov.mvhd.duration);
    assert_eq!(result_moov.traks.len(), 1);

    let result_trak = &result_moov.traks[0];
    let original_trak = &original_moov.traks[0];

    assert_eq!(result_trak.tkhd.track_id, original_trak.tkhd.track_id);
    assert_eq!(result_trak.tkhd.duration, original_trak.tkhd.duration);

    let result_mdhd = &result_trak.mdia.mdhd;
    let original_mdhd = &original_trak.mdia.mdhd;
    assert_eq!(result_mdhd.timescale, original_mdhd.timescale);
    assert_eq!(result_mdhd.duration, original_mdhd.duration);

    // Compare sample counts
    let result_stbl = &result_trak.mdia.minf.stbl;
    let original_stbl = &original_trak.mdia.minf.stbl;

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

    // Compare total duration in ticks
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

    // Compare sample sizes
    match (&result_stbl.sample_size, &original_stbl.sample_size) {
        (SampleSize::Stsz(r), SampleSize::Stsz(o)) => {
            assert_eq!(r.sample_count, o.sample_count);
            let r_sizes: Vec<u32> = if r.sample_size != 0 {
                vec![r.sample_size; r.sample_count as usize]
            } else {
                r.entries.iter().map(|e| e.entry_size).collect()
            };
            let o_sizes: Vec<u32> = if o.sample_size != 0 {
                vec![o.sample_size; o.sample_count as usize]
            } else {
                o.entries.iter().map(|e| e.entry_size).collect()
            };
            assert_eq!(r_sizes, o_sizes);
        }
        _ => panic!("Expected both to be Stsz"),
    }

    // Compare chunk offset counts
    assert_eq!(
        result_stbl.chunk_offset.entries_count(),
        original_stbl.chunk_offset.entries_count(),
    );

    // Compare sync samples
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
fn mux_then_demux_roundtrip() {
    let original_moov = decode_moov(SAMPLE_MP4);
    let stsd_entry = &original_moov.traks[0].mdia.minf.stbl.stsd.entries[0];
    let avc1 = Avc1SampleEntry::decode(stsd_entry.payload()).unwrap();
    let media = MediaDefinition::Video {
        width: avc1.base.width,
        height: avc1.base.height,
        codec: VisualSampleDescription::Avc1(avc1.avcc.avc_config),
    };

    // 1. Create a MoovBox via Muxer
    let mut builder = Muxer::builder(1000).unwrap();
    let track_id = builder.add_track(90000, media).unwrap().build();
    let mut muxer = builder.build().unwrap();

    let s1 = Sample::new(0, Some(66_000_000), Duration::from_millis(33), true, 1000).unwrap();
    let s2 = Sample::new(
        33_000_000,
        Some(33_000_000),
        Duration::from_millis(33),
        false,
        800,
    )
    .unwrap();
    let s3 = Sample::new(
        66_000_000,
        Some(99_000_000),
        Duration::from_millis(33),
        false,
        900,
    )
    .unwrap();

    let mut chunk = Chunk::new(0, s1);
    chunk.try_push_sample(s2).unwrap();
    chunk.try_push_sample(s3).unwrap();
    muxer.add_chunk(track_id, chunk).unwrap();

    let moov = muxer.finalize().unwrap();

    // 2. Demux the MoovBox
    let demuxer = Demuxer::new(&moov).unwrap();
    let tracks: Vec<_> = demuxer.tracks().collect();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].timescale(), 90000);

    let chunks = demuxer.chunks(tracks[0].id()).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data_offset(), 0);
    assert_eq!(chunks[0].sample_count(), 3);

    let samples: Vec<_> = chunks[0].samples().collect();

    // Verify DTS
    assert_eq!(samples[0].dts_ns(), 0);
    assert_eq!(samples[1].dts_ns(), 33_000_000);
    assert_eq!(samples[2].dts_ns(), 66_000_000);

    // Verify sizes
    assert_eq!(samples[0].size(), 1000);
    assert_eq!(samples[1].size(), 800);
    assert_eq!(samples[2].size(), 900);

    // Verify sync flags
    assert!(samples[0].is_sync());
    assert!(!samples[1].is_sync());
    assert!(!samples[2].is_sync());

    // Verify PTS is present (ctts was generated)
    assert!(samples[0].pts_ns().is_some());
}

#[test]
fn fragmented_mux_then_demux_roundtrip() {
    let original_moov = decode_moov(SAMPLE_MP4);
    let stsd_entry = &original_moov.traks[0].mdia.minf.stbl.stsd.entries[0];
    let avc1 = Avc1SampleEntry::decode(stsd_entry.payload()).unwrap();
    let media = MediaDefinition::Video {
        width: avc1.base.width,
        height: avc1.base.height,
        codec: VisualSampleDescription::Avc1(avc1.avcc.avc_config),
    };

    // 1. Build fragmented MP4 via FragmentedMuxer
    let mut builder = FragmentedMuxer::builder(1000).unwrap();
    let track_id = builder
        .add_track(90000, media)
        .unwrap()
        .sample_default_duration(Duration::from_millis(33))
        .sample_default_size(1000)
        .sample_default_flags(SampleFlags::non_sync())
        .build();
    let (init_moov, mut muxer) = builder.build_fragmented().unwrap();

    // Fragment 1: 2 samples
    let s1 = Sample::new(0, None, Duration::from_millis(33), true, 1000).unwrap();
    let s2 = Sample::new(33_000_000, None, Duration::from_millis(33), false, 1000).unwrap();
    let mut chunk1 = Chunk::new(100, s1);
    chunk1.try_push_sample(s2).unwrap();
    muxer.add_chunk(track_id, chunk1).unwrap();
    let moof1 = muxer.flush_fragment().unwrap();

    // Fragment 2: 1 sample
    let s3 = Sample::new(66_000_000, None, Duration::from_millis(33), true, 1000).unwrap();
    let chunk2 = Chunk::new(200, s3);
    muxer.add_chunk(track_id, chunk2).unwrap();
    let moof2 = muxer.flush_fragment().unwrap();

    // 2. Demux via FragmentedDemuxer
    let demuxer = FragmentedDemuxer::new(&init_moov).unwrap();
    let tracks: Vec<_> = demuxer.tracks().collect();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].timescale(), 90000);

    // Fragment 1
    let frag1 = demuxer.read_fragment(&moof1).unwrap();
    let chunks1 = frag1.chunks(track_id).unwrap();
    assert_eq!(chunks1.len(), 1);
    assert_eq!(chunks1[0].sample_count(), 2);

    let samples: Vec<_> = chunks1[0].samples().collect();
    assert_eq!(samples[0].dts_ns(), 0);
    assert!(samples[0].is_sync());
    assert!(!samples[1].is_sync());

    // Fragment 2
    let frag2 = demuxer.read_fragment(&moof2).unwrap();
    let chunks2 = frag2.chunks(track_id).unwrap();
    assert_eq!(chunks2.len(), 1);
    assert_eq!(chunks2[0].sample_count(), 1);
    assert!(chunks2[0].first_sample().is_sync());
}

#[test]
fn fragmented_partial_tracks() {
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

    // 2 tracks, only one gets samples in the fragment
    let mut builder = FragmentedMuxer::builder(1000).unwrap();
    let tid1 = builder.add_track(90000, media1).unwrap().build();
    let _tid2 = builder.add_track(90000, media2).unwrap().build();
    let (init_moov, mut muxer) = builder.build_fragmented().unwrap();

    // Only add chunk to track 1
    let s = Sample::new(0, None, Duration::from_millis(33), true, 500).unwrap();
    let chunk = Chunk::new(0, s);
    muxer.add_chunk(tid1, chunk).unwrap();
    let moof = muxer.flush_fragment().unwrap();

    // Demux
    let demuxer = FragmentedDemuxer::new(&init_moov).unwrap();
    let tracks: Vec<_> = demuxer.tracks().collect();
    assert_eq!(tracks.len(), 2);

    let frag = demuxer.read_fragment(&moof).unwrap();
    let chunks = frag.chunks(tid1).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].sample_count(), 1);
}
