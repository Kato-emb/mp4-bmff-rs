//!

use std::time::Duration;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::prelude::*;

use mp4_bmff_util::multiplex::demux::Demuxer;

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
fn demux_real_file() {
    let moov = decode_moov(SAMPLE_MP4);
    let demuxer = Demuxer::new(&moov).unwrap();

    let tracks: Vec<_> = demuxer.tracks().collect();
    assert_eq!(tracks.len(), 1);

    let track = tracks[0];
    assert_eq!(track.id().get(), 1);

    let chunks = demuxer.chunks(track.id()).unwrap();
    assert!(!chunks.is_empty());

    let total_samples: usize = chunks.iter().map(|c| c.sample_count()).sum();
    assert_eq!(total_samples, 3, "Expected 3 frames in sample.mp4");

    for chunk in chunks {
        for sample in chunk.samples() {
            assert!(sample.size() > 0);
            assert!(sample.duration() > Duration::ZERO);
        }
    }
}
