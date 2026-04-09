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
pub(super) struct Timescale(pub NonZeroU32);

impl Timescale {
    #[cfg(feature = "mux")]
    pub(super) fn nanos_to_ticks(&self, nanos: u64) -> Option<u64> {
        let ts = self.as_u64();
        let secs = nanos.checked_div(1_000_000_000)?;
        let sub_nanos = nanos.checked_rem(1_000_000_000)?;

        secs.checked_mul(ts)?
            .checked_add(sub_nanos.checked_mul(ts)?.checked_div(1_000_000_000)?)
    }

    #[cfg(feature = "mux")]
    pub(super) fn duration_to_ticks(&self, duration: Duration) -> Option<u64> {
        let ts = self.as_u64();
        let secs = duration.as_secs();
        let sub_nanos = u64::from(duration.subsec_nanos());

        secs.checked_mul(ts)?
            .checked_add(sub_nanos.checked_mul(ts)?.checked_div(1_000_000_000)?)
    }

    pub(super) fn as_u32(&self) -> u32 {
        self.0.get()
    }

    fn as_u64(&self) -> u64 {
        u64::from(self.0.get())
    }
}
