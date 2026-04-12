use core::num::NonZeroU32;
use core::time::Duration;

mod data_layout;
mod description;
mod movie;
mod timeline;
mod track;

pub use data_layout::DataLayout;
pub use description::{
    AudioSampleDescription, //
    FontSampleDescription,
    HintSampleDescription,
    MetadataSampleDescription,
    SampleDescription,
    SubtitleSampleDescription,
    TextSampleDescription,
    VisualSampleDescription,
};
pub use movie::Movie;
pub use timeline::Timeline;
pub use track::Track;

/// Unique identifier for a track within an MP4 file, used to associate samples and metadata with the correct track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(NonZeroU32);

impl TrackId {
    /// Creates a new `TrackId` with the given value.
    pub(super) fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }

    pub(super) fn as_u32(&self) -> u32 {
        self.0.get()
    }

    pub(super) fn as_nonzero(&self) -> NonZeroU32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Timescale(pub(super) NonZeroU32);

impl Timescale {
    pub(crate) fn new(timescale: u32) -> Option<Self> {
        NonZeroU32::new(timescale).map(Self)
    }

    pub(crate) fn nanos_to_ticks(&self, nanos: u64) -> Option<u64> {
        let ts = self.as_u64();
        let secs = nanos.checked_div(1_000_000_000)?;
        let sub_nanos = nanos.checked_rem(1_000_000_000)?;

        secs.checked_mul(ts)?
            .checked_add(sub_nanos.checked_mul(ts)?.checked_div(1_000_000_000)?)
    }

    pub(crate) fn duration_to_ticks(&self, duration: Duration) -> Option<u64> {
        let ts = self.as_u64();
        let secs = duration.as_secs();
        let sub_nanos = u64::from(duration.subsec_nanos());

        secs.checked_mul(ts)?
            .checked_add(sub_nanos.checked_mul(ts)?.checked_div(1_000_000_000)?)
    }

    pub(crate) fn rescale_ticks(&self, ticks: u64, target: Timescale) -> Option<u64> {
        let source_ts = u128::from(self.as_u64());
        let target_ts = u128::from(target.as_u64());

        let result = u128::from(ticks)
            .checked_mul(target_ts)?
            .checked_div(source_ts)?;

        u64::try_from(result).ok()
    }

    pub(crate) fn as_u32(&self) -> u32 {
        self.0.get()
    }

    fn as_u64(&self) -> u64 {
        u64::from(self.0.get())
    }
}
