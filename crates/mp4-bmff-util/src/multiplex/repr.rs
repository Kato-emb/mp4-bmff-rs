use alloc::vec::Vec;

use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::types::{I16F16, LanguageCode, Matrix};

use super::context::SampleDescription;

use super::TrackId;

#[derive(Debug, Clone)]
pub(super) struct Movie {
    pub timescale: Timescale,
    pub tracks: Vec<Track>,
}

impl Movie {
    pub(super) fn next_track_id(&self) -> Option<TrackId> {
        let next_id = match self.tracks.iter().map(|t| t.id).max() {
            Some(max_id) => max_id.as_u32().checked_add(1),
            None => Some(1),
        };

        next_id.and_then(TrackId::new)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Track {
    pub id: TrackId,
    pub timescale: Timescale,
    pub language: LanguageCode,
    pub matrix: Matrix,
    pub alternate_group: i16,
    pub descriptions: Vec<SampleDescription>,
    pub samples: Vec<Sample>,
    pub edit_list: Option<Vec<Edit>>,
}

impl Track {
    pub(super) fn media_duration(&self) -> u64 {
        self.samples.iter().map(|s| u64::from(s.delta)).sum()
    }

    pub(super) fn edit_duration(&self) -> Option<u64> {
        self.edit_list
            .as_ref()
            .map(|edits| edits.iter().map(|e| e.segment_duration).sum())
    }

    pub(super) fn primary_description(&self) -> Option<&SampleDescription> {
        self.descriptions.get(0)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Sample {
    pub delta: u32,
    pub is_sync: bool,
    pub composition_time_offset: i64,
}

#[derive(Debug, Clone)]
pub(super) struct Edit {
    pub segment_duration: u64,
    pub media_time: i64,
    pub media_rate: I16F16,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Timescale(pub(super) NonZeroU32);

impl Timescale {
    pub(super) fn new(timescale: u32) -> Option<Self> {
        NonZeroU32::new(timescale).map(Self)
    }

    pub(super) fn nanos_to_ticks(&self, nanos: u64) -> Option<u64> {
        let ts = self.as_u64();
        let secs = nanos.checked_div(1_000_000_000)?;
        let sub_nanos = nanos.checked_rem(1_000_000_000)?;

        secs.checked_mul(ts)?
            .checked_add(sub_nanos.checked_mul(ts)?.checked_div(1_000_000_000)?)
    }

    pub(super) fn duration_to_ticks(&self, duration: Duration) -> Option<u64> {
        let ts = self.as_u64();
        let secs = duration.as_secs();
        let sub_nanos = u64::from(duration.subsec_nanos());

        secs.checked_mul(ts)?
            .checked_add(sub_nanos.checked_mul(ts)?.checked_div(1_000_000_000)?)
    }

    pub(super) fn rescale_ticks(&self, ticks: u64, target: Timescale) -> Option<u64> {
        let source_ts = u128::from(self.as_u64());
        let target_ts = u128::from(target.as_u64());

        let result = u128::from(ticks)
            .checked_mul(target_ts)?
            .checked_div(source_ts)?;

        u64::try_from(result).ok()
    }

    pub(super) fn as_u32(&self) -> u32 {
        self.0.get()
    }

    fn as_u64(&self) -> u64 {
        u64::from(self.0.get())
    }
}
