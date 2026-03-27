//!

use core::num::NonZeroU32;
use core::time::Duration;

use alloc::vec::Vec;
use mp4_bmff::boxes::bmff::{
    ChunkOffset, CttsBox, CttsEntry, SampleSize, StblBox, StscBox, StsdBox, StssBox, StszBox,
    SttsBox,
};

use super::mux::duration_to_ticks;
use crate::{mux::MuxError, track::MediaDefinition};

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

#[derive(Debug)]
pub(crate) struct SampleEntry {
    pub(crate) duration: Duration,
    pub(crate) composition_time_offset: Option<i32>,
    pub(crate) is_sync: bool,
    pub(crate) size: u32,
}

impl SampleEntry {
    fn sample_delta(&self, timescale: NonZeroU32) -> Result<u32, MuxError> {
        duration_to_ticks(self.duration, timescale.get())
            .and_then(|ticks| u32::try_from(ticks).ok())
            .ok_or(MuxError::Overflow)
    }
}

#[derive(Debug)]
pub(crate) struct Chunk {
    pub(crate) offset: u64,
    pub(crate) entries: Vec<SampleEntry>,
}

impl Chunk {}

#[derive(Debug)]
pub(crate) struct Fragment {
    pub(crate) dts_ns: u64,
    pub(crate) data_offset: u64,
    pub(crate) entries: Vec<SampleEntry>,
}

#[derive(Debug)]
pub struct SampleTable {
    pub(crate) chunks: Vec<Chunk>,
}

impl SampleTable {
    pub(crate) fn media_duration(&self) -> Duration {
        self.chunks
            .iter()
            .flat_map(|chunk| chunk.entries.iter())
            .fold(Duration::ZERO, |acc, entry| acc + entry.duration)
    }

    pub(crate) fn build_stbl(
        &self,
        media: &MediaDefinition,
        media_timescale: NonZeroU32,
    ) -> Result<StblBox, MuxError> {
        let stsd = StsdBox {
            entries: alloc::vec![media.to_raw_box()?],
            ..Default::default()
        };

        let mut stts = SttsBox::default();
        let mut stsz = StszBox::default();
        let mut stsc = StscBox::default();
        let mut ctts = CttsBox::default();
        let mut stss = StssBox::default();

        let mut chunk_number: u32 = 0;
        let mut sample_number: u32 = 0;
        let mut offsets: Vec<u64> = Vec::new();

        for chunk in self.chunks.iter() {
            if chunk.entries.is_empty() {
                continue;
            }

            chunk_number += 1;
            stsc.push(chunk_number, chunk.entries.len() as u32, 1);
            offsets.push(chunk.offset);

            for sample in chunk.entries.iter() {
                sample_number += 1;

                // delta
                let sample_delta = sample.sample_delta(media_timescale)?;
                stts.push(sample_delta);

                // size
                stsz.push(sample.size);

                // sync sample
                if sample.is_sync {
                    stss.push(sample_number);
                }

                // composition time offset
                if let Some(offset) = sample.composition_time_offset {
                    if ctts.entries.is_empty() && sample_number > 1 {
                        ctts.push_entry(CttsEntry {
                            sample_count: sample_number - 1,
                            sample_offset: 0,
                        });
                    }

                    ctts.push(offset);
                } else if !ctts.entries.is_empty() {
                    ctts.push(0);
                }
            }
        }

        let sample_size = SampleSize::Stsz(stsz);
        let chunk_offset = ChunkOffset::from(offsets.as_slice());
        let ctts = if ctts.entries.is_empty() {
            None
        } else {
            Some(ctts)
        };

        let stss = if stss.entries.len() == sample_number as usize {
            None
        } else {
            Some(stss)
        };

        Ok(StblBox {
            stsd,
            stts,
            sample_size,
            stsc,
            chunk_offset,
            stdp: None,
            ctts,
            cslg: None,
            stss,
            stsh: None,
            sdtp: None,
            sbgps: Vec::new(),
            sgpds: Vec::new(),
        })
    }
}
