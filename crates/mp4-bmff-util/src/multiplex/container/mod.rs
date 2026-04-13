use core::num::NonZeroU32;
use core::time::Duration;

mod chunk;
mod description;
mod movie;
mod spec;
mod track;

pub use chunk::ChunkLayout;
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
pub use spec::SampleSpec;
pub use track::Track;

/// Unique identifier for a track within an MP4 file, used to associate samples and metadata with the correct track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(NonZeroU32);

impl TrackId {
    /// Creates a new `TrackId` from the given value. Returns `None` if `id` is zero, since track IDs in BMFF must be non-zero.
    pub(crate) fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }

    /// Returns the underlying `u32` value of this track ID.
    pub const fn as_u32(&self) -> u32 {
        self.0.get()
    }

    /// Returns the underlying `NonZeroU32` value of this track ID.
    pub const fn as_nonzero(&self) -> NonZeroU32 {
        self.0
    }
}

/// Number of time units per second used to express durations within a movie or track.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Timescale(pub(super) NonZeroU32);

impl Timescale {
    /// Creates a new `Timescale` from the given value. Returns `None` if `timescale` is zero, since BMFF requires a non-zero timescale.
    pub(crate) fn new(timescale: u32) -> Option<Self> {
        NonZeroU32::new(timescale).map(Self)
    }

    /// Converts a nanosecond duration to a count of ticks in this timescale. Returns `None` on overflow.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn nanos_to_ticks(&self, nanos: u64) -> Option<u64> {
        let ts = self.as_u64();
        let secs = nanos.checked_div(1_000_000_000)?;
        let sub_nanos = nanos.checked_rem(1_000_000_000)?;

        secs.checked_mul(ts)?
            .checked_add(sub_nanos.checked_mul(ts)?.checked_div(1_000_000_000)?)
    }

    /// Converts a [`Duration`] to a count of ticks in this timescale. Returns `None` on overflow.
    #[cfg_attr(not(test), allow(dead_code))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_id_rejects_zero() {
        assert!(TrackId::new(0).is_none());
        assert_eq!(TrackId::new(1).unwrap().as_u32(), 1);
        assert_eq!(TrackId::new(u32::MAX).unwrap().as_u32(), u32::MAX);
    }

    #[test]
    fn track_id_as_nonzero() {
        let id = TrackId::new(42).unwrap();
        assert_eq!(id.as_u32(), 42);
        assert_eq!(id.as_nonzero().get(), 42);
    }

    #[test]
    fn track_id_ordering() {
        let a = TrackId::new(1).unwrap();
        let b = TrackId::new(2).unwrap();
        assert!(a < b);
        assert_eq!(a, TrackId::new(1).unwrap());
    }

    #[test]
    fn timescale_rejects_zero() {
        assert!(Timescale::new(0).is_none());
        assert_eq!(Timescale::new(1000).unwrap().as_u32(), 1000);
    }

    #[test]
    fn timescale_duration_to_ticks() {
        let ts = Timescale::new(1000).unwrap();
        assert_eq!(ts.duration_to_ticks(Duration::from_secs(2)), Some(2000));
        assert_eq!(
            ts.duration_to_ticks(Duration::from_millis(1500)),
            Some(1500)
        );
        // sub-nanosecond precision: 1us at 1MHz = 1 tick
        let mhz = Timescale::new(1_000_000).unwrap();
        assert_eq!(mhz.duration_to_ticks(Duration::from_micros(1)), Some(1));
    }

    #[test]
    fn timescale_nanos_to_ticks() {
        let ts = Timescale::new(1000).unwrap();
        assert_eq!(ts.nanos_to_ticks(1_000_000_000), Some(1000)); // 1s = 1000 ticks
        assert_eq!(ts.nanos_to_ticks(1_500_000), Some(1)); // 1.5ms = 1.5 ticks → 1
    }

    #[test]
    fn timescale_rescale_basic() {
        let src = Timescale::new(1000).unwrap();
        let dst = Timescale::new(48000).unwrap();
        // 1 second in src = 1000 ticks → in dst = 48000 ticks
        assert_eq!(src.rescale_ticks(1000, dst), Some(48000));
        // round-trip
        assert_eq!(dst.rescale_ticks(48000, src), Some(1000));
    }

    #[test]
    fn timescale_rescale_returns_none_on_u64_overflow() {
        // Inner computation uses u128 to avoid intermediate overflow, but the
        // final cast back to u64 must fail when the result no longer fits.
        let src = Timescale::new(1).unwrap();
        let dst = Timescale::new(u32::MAX).unwrap();
        assert_eq!(src.rescale_ticks(u64::MAX / 2, dst), None);
    }

    #[test]
    fn timescale_rescale_handles_large_values_within_u64() {
        // 1 hour at 90kHz rescaled to 1MHz fits in u64
        let src = Timescale::new(90_000).unwrap();
        let dst = Timescale::new(1_000_000).unwrap();
        let one_hour = 3600u64 * 90_000;
        assert_eq!(src.rescale_ticks(one_hour, dst), Some(3600 * 1_000_000));
    }
}
