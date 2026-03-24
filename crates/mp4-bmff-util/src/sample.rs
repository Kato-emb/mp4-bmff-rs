//!

use core::num::NonZeroU32;
use core::time::Duration;

use alloc::vec::Vec;

/// Represents a media sample in an MP4 file, containing metadata and sample data.
#[derive(Debug)]
pub struct Sample<T = Vec<u8>> {
    track_id: NonZeroU32,
    dts_ns: u64,
    pts_ns: Option<i64>,
    duration: Duration,
    is_sync: bool,
    data: T,
}

impl<T> Sample<T> {
    /// Creates a new `Sample` with the given parameters.
    ///
    /// # Panics
    /// - `track_id` must be non-zero.
    /// - The composition time offset is calculated as `pts - dts` and must fit within an `i32`. If it does not, it will default to `0`.
    pub fn new(
        track_id: u32,
        dts_ns: u64,
        pts_ns: Option<i64>,
        duration: Duration,
        is_sync: bool,
        data: T,
    ) -> Self {
        let track_id = NonZeroU32::new(track_id).expect("track_id must be non-zero");

        Self {
            track_id,
            dts_ns,
            pts_ns,
            duration,
            is_sync,
            data,
        }
    }

    /// Returns the track ID of the sample.
    pub fn track_id(&self) -> u32 {
        self.track_id.get()
    }

    /// Returns the decode time of the sample in nanoseconds.
    pub fn dts_ns(&self) -> u64 {
        self.dts_ns
    }

    /// Returns the presentation time of the sample in nanoseconds, if available.
    pub fn pts_ns(&self) -> Option<i64> {
        self.pts_ns
    }

    /// Returns the duration of the sample.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns whether the sample is a sync sample (keyframe).
    pub fn is_sync(&self) -> bool {
        self.is_sync
    }

    /// Returns a reference to the sample data.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Transforms the sample data using the provided function, returning a new `Sample` with the transformed data.
    pub fn map_data<U>(self, f: impl FnOnce(T) -> U) -> Sample<U> {
        Sample {
            track_id: self.track_id,
            dts_ns: self.dts_ns,
            pts_ns: self.pts_ns,
            duration: self.duration,
            is_sync: self.is_sync,
            data: f(self.data),
        }
    }
}

impl<T: AsRef<[u8]>> Sample<T> {
    /// Returns a reference to the sample data as a byte slice.
    pub fn as_ref(&self) -> Sample<&[u8]> {
        Sample {
            track_id: self.track_id,
            dts_ns: self.dts_ns,
            pts_ns: self.pts_ns,
            duration: self.duration,
            is_sync: self.is_sync,
            data: self.data.as_ref(),
        }
    }
}
