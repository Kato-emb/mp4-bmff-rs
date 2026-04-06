//!

use std::time::Duration;

use mp4_bmff::boxes::avc::Avc1SampleEntry;
use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::*;

use mp4_bmff_util::multiplex::mux::Muxer;
use mp4_bmff_util::multiplex::{Chunk, MediaDefinition, Sample, VisualSampleDescription};

const SAMPLE_MP4: &[u8] = include_bytes!("samples/sample.mp4");

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

    let mut chunk1 = Chunk::new(
        0,
        Sample::new(0, None, Duration::from_millis(33), true, 1000).unwrap(),
    );
    chunk1
        .try_push_sample(
            Sample::new(33_000_000, None, Duration::from_millis(33), false, 800).unwrap(),
        )
        .unwrap();
    muxer.add_chunk(tid1, chunk1).unwrap();

    let mut chunk2 = Chunk::new(
        2000,
        Sample::new(0, None, Duration::from_millis(33), true, 500).unwrap(),
    );
    chunk2
        .try_push_sample(
            Sample::new(33_000_000, None, Duration::from_millis(33), true, 500).unwrap(),
        )
        .unwrap();
    muxer.add_chunk(tid2, chunk2).unwrap();

    let moov = muxer.finalize().unwrap();

    assert_eq!(moov.traks.len(), 2);
    assert_eq!(moov.mvhd.next_track_id, 3);
    assert_eq!(moov.traks[0].tkhd.track_id, 1);
    assert_eq!(moov.traks[1].tkhd.track_id, 2);
}
