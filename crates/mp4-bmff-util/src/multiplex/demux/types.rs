use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::SampleFlags;

use crate::multiplex::{Chunk, Track};

/// Per-track data stored inside Demuxer.
#[derive(Debug)]
pub(super) struct DemuxedTrack {
    pub info: Track,
    pub chunks: Vec<Chunk>,
}

/// Track defaults extracted from trex for fragmented demuxing.
#[derive(Debug)]
pub(super) struct TrackDefaults {
    pub default_sample_duration: u32,
    pub default_sample_size: u32,
    pub default_sample_flags: SampleFlags,
}

/// Per-track context stored inside FragmentedDemuxer.
#[derive(Debug)]
pub(super) struct TrackContext {
    pub info: Track,
    pub trex_defaults: TrackDefaults,
}
