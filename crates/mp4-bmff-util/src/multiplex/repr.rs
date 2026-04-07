use core::num::NonZeroU32;

use mp4_bmff::boxes::bmff::SampleFlags;

use super::Chunk;
use super::Track;

pub(super) struct TickScale(pub(super) NonZeroU32);

#[derive(Debug)]
pub(super) struct Movie {
    pub timescale: NonZeroU32,
    pub next_track_id: u32,
    pub tracks: Vec<TrackRepr>,
}

#[derive(Debug)]
pub(super) struct TrackRepr {
    pub track: Track,
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TrackFragmentDefaults {
    pub sample_duration: u32,
    pub sample_size: u32,
    pub sample_flags: SampleFlags,
}
