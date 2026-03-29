//!

use core::num::NonZeroU32;
use core::time::Duration;

use alloc::vec::Vec;
use mp4_bmff::boxes::bmff::{
    ChunkOffset, CttsBox, SampleFlags, SampleSize, StblBox, StcoBox, StscBox, StsdBox, StssBox,
    StszBox, SttsBox, TrunBox, TrunEntry, TrunFlags,
};

use super::mux::duration_to_ticks;
use crate::mux::MuxError;

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
pub(crate) struct SampleMetadata {
    pub(crate) dts_ns: u64,
    pub(crate) pts_ns: Option<i64>,
    pub(crate) duration: Duration,
    pub(crate) is_sync: bool,
    pub(crate) size: u32,
}

impl SampleMetadata {
    pub(crate) fn from_sample<T: AsRef<[u8]>>(sample: &Sample<T>) -> Result<Self, MuxError> {
        let size = u32::try_from(sample.data.as_ref().len()).map_err(|_| MuxError::Overflow)?;

        Ok(SampleMetadata {
            dts_ns: sample.dts_ns,
            pts_ns: sample.pts_ns,
            duration: sample.duration,
            is_sync: sample.is_sync,
            size,
        })
    }

    fn sample_delta(&self, timescale: NonZeroU32) -> Result<u32, MuxError> {
        duration_to_ticks(self.duration, timescale.get())
            .and_then(|ticks| u32::try_from(ticks).ok())
            .ok_or(MuxError::Overflow)
    }

    fn composition_time_offset(&self, media_timescale: NonZeroU32) -> Result<i32, MuxError> {
        match self.pts_ns {
            Some(pts) => {
                let diff_ns = pts
                    .checked_sub(self.dts_ns as i64)
                    .ok_or(MuxError::Overflow)?;
                let ts = media_timescale.get() as i64;
                let ticks = diff_ns.checked_mul(ts).ok_or(MuxError::Overflow)? / 1_000_000_000;
                i32::try_from(ticks).map_err(|_| MuxError::Overflow)
            }
            None => Ok(0),
        }
    }
}

#[derive(Debug)]
pub(crate) struct SampleChunk {
    pub(crate) data_offset: u64,
    pub(crate) entries: Vec<SampleMetadata>,
}

#[derive(Debug)]
pub(super) struct SampleTable {
    pub(super) chunks: Vec<SampleChunk>,
}

impl SampleTable {
    pub(super) fn media_duration(&self) -> Duration {
        self.chunks
            .iter()
            .flat_map(|group| group.entries.iter())
            .fold(Duration::ZERO, |acc, entry| acc + entry.duration)
    }

    pub(super) fn empty() -> Self {
        Self { chunks: Vec::new() }
    }

    pub(super) fn build_stbl(
        &self,
        stsd: StsdBox,
        media_timescale: NonZeroU32,
    ) -> Result<StblBox, MuxError> {
        let mut stts = SttsBox::default();
        let mut stsz = StszBox::default();
        let mut stsc = StscBox::default();
        let mut ctts = CttsBox::default();
        let mut stss = StssBox::default();
        let mut chunk_offset = ChunkOffset::Stco(StcoBox::default());

        let mut has_nonzero_ctts = false;
        let mut chunk_number: u32 = 0;
        let mut sample_number: u32 = 0;

        for chunk in self.chunks.iter() {
            debug_assert!(!chunk.entries.is_empty());

            chunk_number = chunk_number.checked_add(1).ok_or(MuxError::Overflow)?;
            stsc.push(chunk_number, chunk.entries.len() as u32, 1);
            chunk_offset.push(chunk.data_offset);

            for sample in chunk.entries.iter() {
                sample_number = sample_number.checked_add(1).ok_or(MuxError::Overflow)?;

                let sample_delta = sample.sample_delta(media_timescale)?;
                let cto = sample.composition_time_offset(media_timescale)?;
                stts.push(sample_delta);
                stsz.push(sample.size);
                ctts.push(cto);
                has_nonzero_ctts |= cto != 0;

                if sample.is_sync {
                    stss.push(sample_number);
                }
            }
        }

        let sample_size = SampleSize::Stsz(stsz);
        let ctts = if has_nonzero_ctts { Some(ctts) } else { None };
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

    pub(super) fn base_media_decode_time(
        &self,
        media_timescale: NonZeroU32,
    ) -> Result<u64, MuxError> {
        self.chunks
            .first()
            .and_then(|chunk| chunk.entries.first())
            .and_then(|s| duration_to_ticks(Duration::from_nanos(s.dts_ns), media_timescale.get()))
            .ok_or(MuxError::Overflow)
    }

    fn build_trun(
        chunk: &SampleChunk,
        media_timescale: NonZeroU32,
        moof_offset: u64,
        default_duration: Option<u32>,
        default_size: Option<u32>,
        default_flags: Option<SampleFlags>,
    ) -> Result<TrunBox, MuxError> {
        // 1. Convert all samples to timescale units
        let computed: Vec<_> = chunk
            .entries
            .iter()
            .map(|s| {
                Ok((
                    s.sample_delta(media_timescale)?,
                    s.size,
                    if s.is_sync {
                        SampleFlags::sync()
                    } else {
                        SampleFlags::non_sync()
                    },
                    s.composition_time_offset(media_timescale)?,
                ))
            })
            .collect::<Result<_, MuxError>>()?;

        // 2. Decide which per-sample fields to emit
        let emit_duration = match default_duration {
            Some(d) if computed.iter().all(|&(dur, ..)| dur == d) => false,
            _ => true,
        };

        let emit_size = match default_size {
            Some(s) if computed.iter().all(|&(_, size, ..)| size == s) => false,
            _ => true,
        };

        let has_cto = computed.iter().any(|&(.., cto)| cto != 0);
        let signed_cto = computed.iter().any(|&(.., cto)| cto < 0);

        let first_sample_flags = match default_flags {
            Some(df)
                if computed.len() > 1
                    && computed[0].2 != df
                    && computed[1..].iter().all(|c| c.2 == df) =>
            {
                Some(computed[0].2)
            }
            _ => None,
        };
        let emit_flags = first_sample_flags.is_none()
            && !default_flags.is_some_and(|df| computed.iter().all(|c| c.2 == df));

        // 3. Build flags
        let mut trun_flags = TrunFlags::DATA_OFFSET_PRESENT;
        if emit_duration {
            trun_flags |= TrunFlags::SAMPLE_DURATION_PRESENT;
        }
        if emit_size {
            trun_flags |= TrunFlags::SAMPLE_SIZE_PRESENT;
        }
        if first_sample_flags.is_some() {
            trun_flags |= TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT;
        } else if emit_flags {
            trun_flags |= TrunFlags::SAMPLE_FLAGS_PRESENT;
        }
        if has_cto {
            trun_flags |= TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT;
        }

        // 4. Populate TrunBox
        let mut trun = TrunBox::new(trun_flags, signed_cto);

        let relative_offset = chunk
            .data_offset
            .checked_sub(moof_offset)
            .ok_or(MuxError::Overflow)?;
        trun.set_data_offset(i32::try_from(relative_offset).map_err(|_| MuxError::Overflow)?);

        if let Some(fsf) = first_sample_flags {
            trun.set_first_sample_flags(fsf);
        }

        for &(duration, size, flags, cto) in &computed {
            trun.push_entry(TrunEntry {
                duration: emit_duration.then_some(duration),
                size: emit_size.then_some(size),
                flags: emit_flags.then_some(flags),
                composition_time_offset: has_cto.then(|| i64::from(cto)),
            });
        }

        Ok(trun)
    }

    pub(super) fn build_truns(
        &self,
        media_timescale: NonZeroU32,
        moof_offset: u64,
        default_sample_duration: Option<u32>,
        default_sample_size: Option<u32>,
        default_sample_flags: Option<SampleFlags>,
    ) -> Result<Vec<TrunBox>, MuxError> {
        let mut truns = Vec::with_capacity(self.chunks.len());

        for chunk in self.chunks.iter() {
            debug_assert!(!chunk.entries.is_empty());
            let trun = Self::build_trun(
                chunk,
                media_timescale,
                moof_offset,
                default_sample_duration,
                default_sample_size,
                default_sample_flags,
            )?;
            truns.push(trun);
        }

        Ok(truns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TS_90K: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(90_000) };
    const TS_48K: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(48_000) };

    fn meta(
        dts_ns: u64,
        pts_ns: Option<i64>,
        dur_ms: u64,
        is_sync: bool,
        size: u32,
    ) -> SampleMetadata {
        SampleMetadata {
            dts_ns,
            pts_ns,
            duration: Duration::from_millis(dur_ms),
            is_sync,
            size,
        }
    }

    fn table(chunks: Vec<SampleChunk>) -> SampleTable {
        SampleTable { chunks }
    }

    fn chunk(data_offset: u64, entries: Vec<SampleMetadata>) -> SampleChunk {
        SampleChunk {
            data_offset,
            entries,
        }
    }

    // ==================== SampleMetadata tests ====================

    #[test]
    fn sample_metadata_from_sample() {
        let sample = Sample::new(1, 0, None, Duration::from_millis(33), true, vec![0u8; 100]);
        let meta = SampleMetadata::from_sample(&sample).unwrap();
        assert_eq!(meta.dts_ns, 0);
        assert_eq!(meta.pts_ns, None);
        assert_eq!(meta.duration, Duration::from_millis(33));
        assert!(meta.is_sync);
        assert_eq!(meta.size, 100);
    }

    #[test]
    fn sample_delta_90k() {
        let m = meta(0, None, 33, true, 100);
        // 33ms * 90000 / 1000 = 2970
        assert_eq!(m.sample_delta(TS_90K).unwrap(), 2970);
    }

    #[test]
    fn sample_delta_48k() {
        let m = meta(0, None, 1000, true, 100);
        // 1s * 48000 = 48000
        assert_eq!(m.sample_delta(TS_48K).unwrap(), 48000);
    }

    #[test]
    fn composition_time_offset_none_pts() {
        let m = meta(1_000_000, None, 33, true, 100);
        assert_eq!(m.composition_time_offset(TS_90K).unwrap(), 0);
    }

    #[test]
    fn composition_time_offset_positive() {
        // pts = dts + 66ms → CTO = 66ms * 90000 / 1e9 = 5940 ticks
        let dts_ns = 100_000_000;
        let pts_ns = Some(166_000_000);
        let m = meta(dts_ns, pts_ns, 33, false, 200);
        assert_eq!(m.composition_time_offset(TS_90K).unwrap(), 5940);
    }

    #[test]
    fn composition_time_offset_negative() {
        // pts = dts - 33ms → CTO = -33ms * 90000 / 1e9 = -2970
        let dts_ns = 100_000_000;
        let pts_ns = Some(67_000_000);
        let m = meta(dts_ns, pts_ns, 33, false, 200);
        assert_eq!(m.composition_time_offset(TS_90K).unwrap(), -2970);
    }

    // ==================== SampleTable::media_duration tests ====================

    #[test]
    fn media_duration_empty() {
        let t = SampleTable::empty();
        assert_eq!(t.media_duration(), Duration::ZERO);
    }

    #[test]
    fn media_duration_sums_across_chunks() {
        let t = table(vec![
            chunk(
                0,
                vec![meta(0, None, 33, true, 100), meta(0, None, 33, false, 200)],
            ),
            chunk(1000, vec![meta(0, None, 34, false, 150)]),
        ]);
        assert_eq!(t.media_duration(), Duration::from_millis(100));
    }

    // ==================== SampleTable::build_stbl tests ====================

    #[test]
    fn build_stbl_single_chunk_all_sync() {
        let t = table(vec![chunk(
            0,
            vec![
                meta(0, None, 33, true, 100),
                meta(33_000_000, None, 33, true, 200),
                meta(66_000_000, None, 34, true, 150),
            ],
        )]);

        let stbl = t.build_stbl(StsdBox::default(), TS_90K).unwrap();

        // stts: 2 entries (2x2970, 1x3060)
        assert_eq!(stbl.stts.entries.len(), 2);
        assert_eq!(stbl.stts.entries[0].sample_count, 2);
        assert_eq!(stbl.stts.entries[0].sample_delta, 2970);
        assert_eq!(stbl.stts.entries[1].sample_count, 1);
        assert_eq!(stbl.stts.entries[1].sample_delta, 3060);

        // stsz: 3 entries
        let SampleSize::Stsz(ref stsz) = stbl.sample_size else {
            panic!("expected Stsz");
        };
        assert_eq!(stsz.sample_count, 3);
        assert_eq!(stsz.entries[0].entry_size, 100);
        assert_eq!(stsz.entries[1].entry_size, 200);
        assert_eq!(stsz.entries[2].entry_size, 150);

        // stsc: 1 chunk with 3 samples
        assert_eq!(stbl.stsc.entries.len(), 1);
        assert_eq!(stbl.stsc.entries[0].first_chunk, 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 3);

        // chunk_offset: 1 entry
        assert_eq!(stbl.chunk_offset.entries_count(), 1);

        // ctts: None (all CTO are 0)
        assert!(stbl.ctts.is_none());

        // stss: None (all samples are sync)
        assert!(stbl.stss.is_none());
    }

    #[test]
    fn build_stbl_with_ctts() {
        let t = table(vec![chunk(
            0,
            vec![
                meta(0, Some(66_000_000), 33, true, 100),
                meta(33_000_000, Some(33_000_000), 33, false, 200),
            ],
        )]);

        let stbl = t.build_stbl(StsdBox::default(), TS_90K).unwrap();

        // ctts should be present because pts != dts
        let ctts = stbl.ctts.as_ref().expect("ctts should be present");
        assert_eq!(ctts.entries.len(), 2);
        // sample 0: CTO = (66M - 0) * 90000 / 1e9 = 5940
        assert_eq!(ctts.entries[0].sample_offset, 5940);
        // sample 1: CTO = (33M - 33M) * 90000 / 1e9 = 0
        assert_eq!(ctts.entries[1].sample_offset, 0);
    }

    #[test]
    fn build_stbl_with_stss() {
        let t = table(vec![chunk(
            0,
            vec![
                meta(0, None, 33, true, 100),
                meta(33_000_000, None, 33, false, 200),
                meta(66_000_000, None, 33, false, 150),
            ],
        )]);

        let stbl = t.build_stbl(StsdBox::default(), TS_90K).unwrap();

        // stss should be present because not all samples are sync
        let stss = stbl.stss.as_ref().expect("stss should be present");
        assert_eq!(stss.entries.len(), 1);
        assert_eq!(stss.entries[0].sample_number, 1);
    }

    #[test]
    fn build_stbl_multiple_chunks() {
        let t = table(vec![
            chunk(0, vec![meta(0, None, 33, true, 100)]),
            chunk(200, vec![meta(33_000_000, None, 33, true, 200)]),
        ]);

        let stbl = t.build_stbl(StsdBox::default(), TS_90K).unwrap();

        // stsc: 2 chunks, each with 1 sample (deduplicated to 1 entry)
        assert_eq!(stbl.stsc.entries.len(), 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 1);

        // chunk_offset: 2 entries
        assert_eq!(stbl.chunk_offset.entries_count(), 2);
    }

    // ==================== SampleTable::base_media_decode_time tests ====================

    #[test]
    fn base_media_decode_time_first_sample() {
        // dts_ns = 1_000_000_000 (1s) → ticks = 90000
        let t = table(vec![chunk(
            0,
            vec![meta(1_000_000_000, None, 33, true, 100)],
        )]);
        assert_eq!(t.base_media_decode_time(TS_90K).unwrap(), 90_000);
    }

    #[test]
    fn base_media_decode_time_empty_table() {
        let t = SampleTable::empty();
        assert!(t.base_media_decode_time(TS_90K).is_err());
    }

    // ==================== SampleTable::build_truns tests ====================

    #[test]
    fn build_truns_all_defaults_match() {
        // All samples match defaults → no per-sample fields, only data_offset
        let t = table(vec![chunk(
            1000,
            vec![
                meta(0, None, 33, false, 500),
                meta(33_000_000, None, 33, false, 500),
            ],
        )]);

        let truns = t
            .build_truns(TS_90K, 0, Some(2970), Some(500), Some(SampleFlags::non_sync()))
            .unwrap();

        assert_eq!(truns.len(), 1);
        let trun = &truns[0];

        let flags = trun.flags();
        assert!(flags.contains(TrunFlags::DATA_OFFSET_PRESENT));
        assert!(!flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT));
        assert!(!flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT));
        assert!(!flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
        assert!(!flags.contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
        assert!(!flags.contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT));

        assert_eq!(trun.data_offset(), Some(1000));
        assert_eq!(trun.sample_count(), 2);
    }

    #[test]
    fn build_truns_no_defaults() {
        // No defaults → all per-sample fields present
        let t = table(vec![chunk(
            500,
            vec![
                meta(0, None, 33, true, 100),
                meta(33_000_000, None, 34, false, 200),
            ],
        )]);

        let truns = t.build_truns(TS_90K, 0, None, None, None).unwrap();
        assert_eq!(truns.len(), 1);

        let flags = truns[0].flags();
        assert!(flags.contains(TrunFlags::DATA_OFFSET_PRESENT));
        assert!(flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT));
        assert!(flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT));
        assert!(flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT));

        let entries: Vec<_> = truns[0].entries().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].duration, Some(2970));
        assert_eq!(entries[0].size, Some(100));
        assert_eq!(entries[0].flags, Some(SampleFlags::sync()));
        assert_eq!(entries[1].duration, Some(3060));
        assert_eq!(entries[1].size, Some(200));
        assert_eq!(entries[1].flags, Some(SampleFlags::non_sync()));
    }

    #[test]
    fn build_truns_first_sample_flags() {
        // First sample is sync (differs from default non_sync), rest match default
        let t = table(vec![chunk(
            100,
            vec![
                meta(0, None, 33, true, 500),
                meta(33_000_000, None, 33, false, 500),
                meta(66_000_000, None, 33, false, 500),
            ],
        )]);

        let truns = t
            .build_truns(TS_90K, 0, Some(2970), Some(500), Some(SampleFlags::non_sync()))
            .unwrap();

        let flags = truns[0].flags();
        assert!(flags.contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
        assert!(!flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
        assert_eq!(truns[0].first_sample_flags(), Some(SampleFlags::sync()));
    }

    #[test]
    fn build_truns_per_sample_flags_when_multiple_differ() {
        // Samples 0 and 2 differ from default → per-sample flags (not first-only)
        let t = table(vec![chunk(
            100,
            vec![
                meta(0, None, 33, true, 500),
                meta(33_000_000, None, 33, false, 500),
                meta(66_000_000, None, 33, true, 500),
            ],
        )]);

        let truns = t
            .build_truns(TS_90K, 0, Some(2970), Some(500), Some(SampleFlags::non_sync()))
            .unwrap();

        let flags = truns[0].flags();
        assert!(flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT));
        assert!(!flags.contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT));
    }

    #[test]
    fn build_truns_with_positive_cto() {
        let t = table(vec![chunk(
            200,
            vec![meta(0, Some(66_000_000), 33, true, 100)],
        )]);

        let truns = t.build_truns(TS_90K, 0, None, None, None).unwrap();

        let flags = truns[0].flags();
        assert!(flags.contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT));
        assert_eq!(truns[0].version(), 0); // positive only → version 0

        let entries: Vec<_> = truns[0].entries().collect();
        assert_eq!(entries[0].composition_time_offset, Some(5940));
    }

    #[test]
    fn build_truns_with_negative_cto() {
        // pts < dts → negative CTO → version 1
        let t = table(vec![chunk(
            200,
            vec![meta(100_000_000, Some(67_000_000), 33, true, 100)],
        )]);

        let truns = t.build_truns(TS_90K, 0, None, None, None).unwrap();

        assert_eq!(truns[0].version(), 1); // signed CTO → version 1

        let entries: Vec<_> = truns[0].entries().collect();
        assert_eq!(entries[0].composition_time_offset, Some(-2970));
    }

    #[test]
    fn build_truns_data_offset_relative_to_moof() {
        let t = table(vec![chunk(1500, vec![meta(0, None, 33, true, 100)])]);

        let truns = t.build_truns(TS_90K, 500, None, None, None).unwrap();
        // data_offset = 1500 - 500 = 1000
        assert_eq!(truns[0].data_offset(), Some(1000));
    }

    #[test]
    fn build_truns_data_offset_underflow() {
        // moof_offset > data_offset → error
        let t = table(vec![chunk(100, vec![meta(0, None, 33, true, 100)])]);

        let result = t.build_truns(TS_90K, 500, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn build_truns_multiple_chunks() {
        let t = table(vec![
            chunk(100, vec![meta(0, None, 33, true, 100)]),
            chunk(300, vec![meta(33_000_000, None, 33, false, 200)]),
        ]);

        let truns = t.build_truns(TS_90K, 0, None, None, None).unwrap();
        assert_eq!(truns.len(), 2);
        assert_eq!(truns[0].data_offset(), Some(100));
        assert_eq!(truns[1].data_offset(), Some(300));
    }

    #[test]
    fn build_truns_no_cto_when_all_zero() {
        let t = table(vec![chunk(
            100,
            vec![
                meta(0, Some(0), 33, true, 100),
                meta(33_000_000, Some(33_000_000), 33, true, 100),
            ],
        )]);

        let truns = t.build_truns(TS_90K, 0, None, None, None).unwrap();

        let flags = truns[0].flags();
        assert!(!flags.contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT));
    }

    // ==================== Sample public API tests ====================

    #[test]
    #[should_panic(expected = "track_id must be non-zero")]
    fn sample_zero_track_id_panics() {
        Sample::new(0, 0, None, Duration::ZERO, true, vec![0u8]);
    }

    #[test]
    fn sample_accessors() {
        let s = Sample::new(
            5,
            1000,
            Some(2000),
            Duration::from_millis(33),
            false,
            vec![1, 2, 3],
        );
        assert_eq!(s.track_id(), 5);
        assert_eq!(s.dts_ns(), 1000);
        assert_eq!(s.pts_ns(), Some(2000));
        assert_eq!(s.duration(), Duration::from_millis(33));
        assert!(!s.is_sync());
        assert_eq!(s.data(), &vec![1, 2, 3]);
    }

    #[test]
    fn sample_map_data() {
        let s = Sample::new(1, 0, None, Duration::ZERO, true, vec![1, 2, 3]);
        let mapped = s.map_data(|d| d.len());
        assert_eq!(*mapped.data(), 3);
        assert_eq!(mapped.track_id(), 1);
    }

    #[test]
    fn sample_as_ref() {
        let s = Sample::new(1, 0, None, Duration::ZERO, true, vec![10, 20]);
        let r = s.as_ref();
        assert_eq!(*r.data(), [10, 20]);
    }
}
