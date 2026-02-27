use core::iter::Peekable;

use mp4_bmff::boxes::bmff::{
    CttsEntry, SampleFlags, SdtpEntry, StblBoxView, StcoEntry, StscEntry, StssEntry, StszEntry,
    SttsEntry, TrafBoxView, TrunEntry,
};

#[cfg(feature = "alloc")]
use mp4_bmff::boxes::bmff::{StblBox, TrafBox};

use super::{Chunk, Sample};

struct SampleResolver<Stts, Ctts, Stsz, Stss, Sdtp>
where
    Stts: Iterator,
    Ctts: Iterator,
    Stss: Iterator,
{
    stts: RunLengthIter<Stts>,
    ctts: Option<RunLengthIter<Ctts>>,
    stsz: Stsz,
    stss: Option<Peekable<Stss>>,
    sdtp: Option<Sdtp>,

    sample_number: u32,
}

impl<Stts, Ctts, Stsz, Stss, Sdtp> SampleResolver<Stts, Ctts, Stsz, Stss, Sdtp>
where
    Stts: Iterator<Item = SttsEntry>,
    Ctts: Iterator<Item = CttsEntry>,
    Stsz: Iterator<Item = StszEntry>,
    Stss: Iterator<Item = StssEntry>,
    Sdtp: Iterator<Item = SdtpEntry>,
{
    fn next_sample(&mut self) -> Option<Sample> {
        let duration = self.stts.next_delta()?;
        let composition_time_offset = self
            .ctts
            .as_mut()
            .and_then(|c| c.next_offset())
            .unwrap_or(0);
        let size = self.stsz.next()?.entry_size;

        let is_sync = match &mut self.stss {
            Some(stss) => {
                if stss
                    .peek()
                    .map_or(false, |e| e.sample_number == self.sample_number + 1)
                {
                    stss.next();
                    true
                } else {
                    false
                }
            }
            None => true,
        };

        let mut sample = Sample {
            size,
            duration,
            composition_time_offset,
            is_sync,
            dependency: None,
        };

        if let Some(sdtp_entry) = self.sdtp.as_mut().and_then(|s| s.next()) {
            let sample_flags = if is_sync {
                SampleFlags::sync()
            } else {
                SampleFlags::non_sync()
            }
            .with_is_leading(sdtp_entry.is_leading)
            .with_sample_depends_on(sdtp_entry.sample_depends_on)
            .with_sample_is_depended_on(sdtp_entry.sample_is_depended_on)
            .with_sample_has_redundancy(sdtp_entry.sample_has_redundancy);

            sample.dependency = Some(sample_flags);
        }

        self.sample_number += 1;
        Some(sample)
    }
}

struct ChunkInfo {
    /// Index into `stsd` (1-based).
    description_index: u32,
    /// Byte offset of the chunk in the file.
    offset: u64,
    /// Number of samples in this chunk.
    sample_count: u32,
}

struct ChunkResolver<Stsc, Stco>
where
    Stsc: Iterator,
{
    stsc: Peekable<Stsc>,
    stco: Stco,

    current_chunk: u32,
    current: Option<Stsc::Item>,
}

impl<Stsc, Stco> ChunkResolver<Stsc, Stco>
where
    Stsc: Iterator<Item = StscEntry>,
    Stco: Iterator<Item = StcoEntry>,
{
    fn next_chunk(&mut self) -> Option<ChunkInfo> {
        let stco_entry = self.stco.next()?;
        self.current_chunk += 1;

        if self
            .stsc
            .peek()
            .is_some_and(|e| e.first_chunk <= self.current_chunk)
        {
            let entry = self.stsc.next().unwrap();
            self.current = Some(entry);
        }

        let stsc_entry = self.current?;

        Some(ChunkInfo {
            description_index: stsc_entry.sample_description_index,
            offset: u64::from(stco_entry.chunk_offset),
            sample_count: stsc_entry.samples_per_chunk,
        })
    }
}

impl<Stsc, Stco> Iterator for ChunkResolver<Stsc, Stco>
where
    Stsc: Iterator<Item = StscEntry>,
    Stco: Iterator<Item = StcoEntry>,
{
    type Item = ChunkInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let stco_entry = self.stco.next()?;
        self.current_chunk += 1;

        if self
            .stsc
            .peek()
            .is_some_and(|e| e.first_chunk <= self.current_chunk)
        {
            let entry = self.stsc.next().unwrap();
            self.current = Some(entry);
        }

        let stsc_entry = self.current?;

        Some(ChunkInfo {
            description_index: stsc_entry.sample_description_index,
            offset: u64::from(stco_entry.chunk_offset),
            sample_count: stsc_entry.samples_per_chunk,
        })
    }
}

/// Extension trait for sample tables.
pub trait SampleTableExt {
    /// Returns an iterator over all samples in the table, with metadata resolved from all relevant boxes.
    fn resolved_samples(&self) -> Result<impl Iterator<Item = Sample> + '_, mp4_bmff::Error>;

    #[cfg(feature = "alloc")]
    fn resolved_chunks(&self) -> Result<impl Iterator<Item = Chunk> + '_, mp4_bmff::Error>;
}

impl SampleTableExt for StblBoxView<'_> {
    fn resolved_samples(&self) -> Result<impl Iterator<Item = Sample> + '_, mp4_bmff::Error> {
        let stts = RunLengthIter::new(self.stts()?.entries());
        let ctts = self.ctts()?.map(|ctts| RunLengthIter::new(ctts.entries()));
        let stsz = self.stsz()?.entries();
        let stss = self.stss()?.map(|stss| stss.entries().peekable());
        let sdtp = self.sdtp()?.map(|sdtp| sdtp.entries());

        let stsc = self.stsc()?.entries();
        let stco = self.stco()?.entries();
        let chunks = ChunkIter {
            stsc: stsc.peekable(),
            stco,
            current_chunk: 0,
            current: None,
        };

        Ok(StblSampleIter {
            stts,
            ctts,
            stsz,
            stss,
            sdtp,
            chunks,
            decode_time: 0,
            sample_number: 0,
            remaining_in_chunk: 0,
            description_index: 0,
            chunk_offset: 0,
            offset_in_chunk: 0,
        })
    }

    #[cfg(feature = "alloc")]
    fn resolved_chunks(&self) -> Result<impl Iterator<Item = Chunk> + '_, mp4_bmff::Error> {
        Ok(Vec::new().into_iter()) // TODO
    }
}

#[cfg(feature = "alloc")]
impl SampleTableExt for StblBox {
    fn resolved_samples(&self) -> Result<impl Iterator<Item = Sample> + '_, mp4_bmff::Error> {
        let stts = RunLengthIter::new(self.stts.entries.iter().copied());
        let ctts = self
            .ctts
            .as_ref()
            .map(|ctts| RunLengthIter::new(ctts.entries.iter().copied()));
        let stsz = self.stsz.entries.iter().copied();
        let stss = self
            .stss
            .as_ref()
            .map(|stss| stss.entries.iter().copied().peekable());
        let sdtp = self.sdtp.as_ref().map(|sdtp| sdtp.entries.iter().copied());

        let stsc = self.stsc.entries.iter().copied();
        let stco = self.stco.entries.iter().copied();
        let chunks = ChunkIter {
            stsc: stsc.peekable(),
            stco,
            current_chunk: 0,
            current: None,
        };

        Ok(StblSampleIter {
            stts,
            ctts,
            stsz,
            stss,
            sdtp,
            chunks,
            decode_time: 0,
            sample_number: 0,
            remaining_in_chunk: 0,
            description_index: 0,
            chunk_offset: 0,
            offset_in_chunk: 0,
        })
    }

    #[cfg(feature = "alloc")]
    fn resolved_chunks(&self) -> Result<impl Iterator<Item = Chunk> + '_, mp4_bmff::Error> {
        Ok(Vec::new().into_iter()) // TODO
    }
}

impl SampleTableExt for TrafBoxView<'_> {
    fn resolved_samples(
        &self,
    ) -> Result<impl Iterator<Item = ResolvedSample> + '_, mp4_bmff::Error> {
        let tfhd = self.tfhd()?;
        let base_decode_time = self.tfdt()?.map_or(0, |t| t.base_media_decode_time);
        let base_offset = tfhd.base_data_offset.unwrap_or(0);

        let truns = self.truns().filter_map(|r| {
            let trun = r.ok()?;
            let header = TrunHeader {
                data_offset: trun.data_offset,
                first_sample_flags: trun.first_sample_flags,
            };
            let samples = trun.samples().filter_map(|r| r.ok());
            Some((header, samples))
        });

        Ok(TrafSampleIter {
            truns,
            current_samples: None,
            first_sample_flags: None,
            is_first_in_trun: false,
            default_duration: tfhd.default_sample_duration.unwrap_or(0),
            default_size: tfhd.default_sample_size.unwrap_or(0),
            default_flags: tfhd.default_sample_flags,
            description_index: tfhd.sample_description_index.unwrap_or(1),
            decode_time: base_decode_time,
            base_offset,
            current_offset: base_offset,
        })
    }

    #[cfg(feature = "alloc")]
    fn resolved_chunks(&self) -> Result<impl Iterator<Item = Chunk> + '_, mp4_bmff::Error> {
        Ok(Vec::new().into_iter()) // TODO
    }
}

#[cfg(feature = "alloc")]
impl SampleTableExt for TrafBox {
    fn resolved_samples(
        &self,
    ) -> Result<impl Iterator<Item = ResolvedSample> + '_, mp4_bmff::Error> {
        let tfhd = &self.tfhd;
        let base_decode_time = self.tfdt.as_ref().map_or(0, |t| t.base_media_decode_time);
        let base_offset = tfhd.base_data_offset.unwrap_or(0);

        let truns = self.truns.iter().map(|trun| {
            let header = TrunHeader {
                data_offset: trun.data_offset,
                first_sample_flags: trun.first_sample_flags,
            };
            let samples = trun.samples.iter().copied();
            (header, samples)
        });

        Ok(TrafSampleIter {
            truns,
            current_samples: None,
            first_sample_flags: None,
            is_first_in_trun: false,
            default_duration: tfhd.default_sample_duration.unwrap_or(0),
            default_size: tfhd.default_sample_size.unwrap_or(0),
            default_flags: tfhd.default_sample_flags,
            description_index: tfhd.sample_description_index.unwrap_or(1),
            decode_time: base_decode_time,
            base_offset,
            current_offset: base_offset,
        })
    }

    #[cfg(feature = "alloc")]
    fn resolved_chunks(&self) -> Result<impl Iterator<Item = Chunk> + '_, mp4_bmff::Error> {
        Ok(Vec::new().into_iter()) // TODO
    }
}

struct StblSampleIter<Stts, Ctts, Stsc, Stsz, Stco, Stss, Sdtp>
where
    Stts: Iterator,
    Ctts: Iterator,
    Stss: Iterator,
    Stsc: Iterator,
{
    stts: RunLengthIter<Stts>,
    ctts: Option<RunLengthIter<Ctts>>,
    stsz: Stsz,
    stss: Option<Peekable<Stss>>,
    sdtp: Option<Sdtp>,
    chunks: ChunkIter<Stsc, Stco>,

    decode_time: u64,
    sample_number: u32,
    remaining_in_chunk: u32,
    description_index: u32,
    chunk_offset: u64,
    offset_in_chunk: u64,
}

impl<Stts, Ctts, Stsc, Stsz, Stco, Stss, Sdtp> Iterator
    for StblSampleIter<Stts, Ctts, Stsc, Stsz, Stco, Stss, Sdtp>
where
    Stts: Iterator<Item = SttsEntry>,
    Ctts: Iterator<Item = CttsEntry>,
    Stsc: Iterator<Item = StscEntry>,
    Stsz: Iterator<Item = StszEntry>,
    Stco: Iterator<Item = StcoEntry>,
    Stss: Iterator<Item = StssEntry>,
    Sdtp: Iterator<Item = SdtpEntry>,
{
    type Item = ResolvedSample;

    fn next(&mut self) -> Option<Self::Item> {
        let duration = self.stts.next_delta()?;
        let composition_time_offset = self
            .ctts
            .as_mut()
            .and_then(|c| c.next_offset())
            .unwrap_or(0);
        let size = self.stsz.next()?.entry_size;

        let is_sync = match &mut self.stss {
            Some(stss) => {
                if stss
                    .peek()
                    .map_or(false, |e| e.sample_number == self.sample_number + 1)
                {
                    stss.next();
                    true
                } else {
                    false
                }
            }
            None => true,
        };

        let mut sample = Sample {
            size,
            duration,
            composition_time_offset,
            is_sync,
            dependency: None,
        };

        if let Some(sdtp_entry) = self.sdtp.as_mut().and_then(|s| s.next()) {
            let sample_flags = if is_sync {
                SampleFlags::sync()
            } else {
                SampleFlags::non_sync()
            }
            .with_is_leading(sdtp_entry.is_leading)
            .with_sample_depends_on(sdtp_entry.sample_depends_on)
            .with_sample_is_depended_on(sdtp_entry.sample_is_depended_on)
            .with_sample_has_redundancy(sdtp_entry.sample_has_redundancy);

            sample.dependency = Some(sample_flags);
        }

        // If we've exhausted the current chunk, move to the next one.
        if self.remaining_in_chunk == 0 {
            let chunk_info = self.chunks.next()?;
            self.description_index = chunk_info.description_index;
            self.chunk_offset = chunk_info.offset;
            self.remaining_in_chunk = chunk_info.sample_count;
            self.offset_in_chunk = 0;
        }

        let offset = self.chunk_offset + self.offset_in_chunk;

        let resolved = ResolvedSample {
            sample,
            description_index: self.description_index,
            offset,
            decode_time: self.decode_time,
        };

        self.offset_in_chunk += u64::from(size);
        self.remaining_in_chunk -= 1;
        self.sample_number += 1;
        self.decode_time += u64::from(duration);

        Some(resolved)
    }
}

// =============================================================================
// RunLengthIter — expands run-length entries one sample at a time
// =============================================================================

pub(super) struct RunLengthIter<I: Iterator> {
    inner: I,
    remaining: u32,
    current: Option<I::Item>,
}

impl<I> RunLengthIter<I>
where
    I: Iterator,
    I::Item: Copy,
{
    pub(super) fn new(iter: I) -> Self {
        Self {
            inner: iter,
            remaining: 0,
            current: None,
        }
    }
}

impl<I: Iterator<Item = SttsEntry>> RunLengthIter<I> {
    pub(super) fn next_delta(&mut self) -> Option<u32> {
        if self.remaining == 0 {
            let entry = self.inner.next()?;
            self.remaining = entry.sample_count;
            self.current = Some(entry);
        }
        self.remaining -= 1;
        self.current.map(|e| e.sample_delta)
    }
}

impl<I: Iterator<Item = CttsEntry>> RunLengthIter<I> {
    pub(super) fn next_offset(&mut self) -> Option<i32> {
        if self.remaining == 0 {
            let entry = self.inner.next()?;
            self.remaining = entry.sample_count;
            self.current = Some(entry);
        }
        self.remaining -= 1;
        self.current.map(|e| e.sample_offset)
    }
}

struct ChunkIter<Stsc: Iterator, Stco> {
    stsc: Peekable<Stsc>,
    stco: Stco,

    current_chunk: u32,
    current: Option<Stsc::Item>,
}

impl<Stsc, Stco> Iterator for ChunkIter<Stsc, Stco>
where
    Stsc: Iterator<Item = StscEntry>,
    Stco: Iterator<Item = StcoEntry>,
{
    type Item = ChunkInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let stco_entry = self.stco.next()?;
        self.current_chunk += 1;

        if self
            .stsc
            .peek()
            .is_some_and(|e| e.first_chunk <= self.current_chunk)
        {
            let entry = self.stsc.next().unwrap();
            self.current = Some(entry);
        }

        let stsc_entry = self.current?;

        Some(ChunkInfo {
            description_index: stsc_entry.sample_description_index,
            offset: u64::from(stco_entry.chunk_offset),
            sample_count: stsc_entry.samples_per_chunk,
        })
    }
}

struct TrunHeader {
    data_offset: Option<i32>,
    first_sample_flags: Option<SampleFlags>,
}

struct TrafSampleIter<Truns, Samples> {
    truns: Truns,
    current_samples: Option<Samples>,

    // per-trun state
    first_sample_flags: Option<SampleFlags>,
    is_first_in_trun: bool,

    // tfhd defaults
    default_duration: u32,
    default_size: u32,
    default_flags: Option<SampleFlags>,
    description_index: u32,

    // cumulative state
    decode_time: u64,
    base_offset: u64,
    current_offset: u64,
}

impl<Truns, Samples> Iterator for TrafSampleIter<Truns, Samples>
where
    Truns: Iterator<Item = (TrunHeader, Samples)>,
    Samples: Iterator<Item = TrunEntry>,
{
    type Item = ResolvedSample;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next entry from current trun.
            if let Some(entry) = self.current_samples.as_mut().and_then(|s| s.next()) {
                // Resolve flags: first_sample_flags > per-sample flags > default_flags.
                let flags = if self.is_first_in_trun {
                    self.is_first_in_trun = false;
                    self.first_sample_flags
                        .or(entry.flags)
                        .or(self.default_flags)
                } else {
                    entry.flags.or(self.default_flags)
                };

                let duration = entry.duration.unwrap_or(self.default_duration);
                let size = entry.size.unwrap_or(self.default_size);
                let is_sync = flags.map_or(true, |f| f.is_sync());
                let composition_time_offset = entry.composition_time_offset.unwrap_or(0);

                let offset = self.current_offset;

                let sample = Sample {
                    size,
                    duration,
                    composition_time_offset,
                    is_sync,
                    dependency: flags,
                };

                let resolved = ResolvedSample {
                    sample,
                    description_index: self.description_index,
                    offset,
                    decode_time: self.decode_time,
                };

                self.current_offset += u64::from(size);
                self.decode_time += u64::from(duration);

                return Some(resolved);
            }

            // Current trun exhausted, advance to next.
            let (header, samples) = self.truns.next()?;
            if let Some(data_offset) = header.data_offset {
                self.current_offset = self.base_offset.wrapping_add_signed(i64::from(data_offset));
            }
            // If no data_offset, current_offset continues sequentially.
            self.first_sample_flags = header.first_sample_flags;
            self.is_first_in_trun = true;
            self.current_samples = Some(samples);
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────

    fn make_stbl_iter(
        stts: Vec<SttsEntry>,
        ctts: Option<Vec<CttsEntry>>,
        stsz: Vec<StszEntry>,
        stsc: Vec<StscEntry>,
        stco: Vec<StcoEntry>,
        stss: Option<Vec<StssEntry>>,
    ) -> impl Iterator<Item = ResolvedSample> {
        StblSampleIter {
            stts: RunLengthIter::new(stts.into_iter()),
            ctts: ctts.map(|c| RunLengthIter::new(c.into_iter())),
            stsz: stsz.into_iter(),
            stss: stss.map(|s| s.into_iter().peekable()),
            sdtp: None::<core::iter::Empty<SdtpEntry>>,
            chunks: ChunkIter {
                stsc: stsc.into_iter().peekable(),
                stco: stco.into_iter(),
                current_chunk: 0,
                current: None,
            },
            decode_time: 0,
            sample_number: 0,
            remaining_in_chunk: 0,
            description_index: 0,
            chunk_offset: 0,
            offset_in_chunk: 0,
        }
    }

    fn make_traf_iter(
        truns: Vec<(TrunHeader, Vec<TrunEntry>)>,
        default_duration: u32,
        default_size: u32,
        default_flags: Option<SampleFlags>,
        description_index: u32,
        base_offset: u64,
        decode_time: u64,
    ) -> impl Iterator<Item = ResolvedSample> {
        let truns = truns
            .into_iter()
            .map(|(header, samples)| (header, samples.into_iter()));
        TrafSampleIter {
            truns,
            current_samples: None,
            first_sample_flags: None,
            is_first_in_trun: false,
            default_duration,
            default_size,
            default_flags,
            description_index,
            decode_time,
            base_offset,
            current_offset: base_offset,
        }
    }

    fn entry(duration: u32, size: u32) -> TrunEntry {
        TrunEntry {
            duration: Some(duration),
            size: Some(size),
            flags: None,
            composition_time_offset: None,
        }
    }

    // ── RunLengthIter ───────────────────────────────────────────────────

    #[test]
    fn run_length_stts_expansion() {
        let entries = vec![
            SttsEntry {
                sample_count: 3,
                sample_delta: 1024,
            },
            SttsEntry {
                sample_count: 2,
                sample_delta: 512,
            },
        ];
        let mut iter = RunLengthIter::new(entries.into_iter());

        assert_eq!(iter.next_delta(), Some(1024));
        assert_eq!(iter.next_delta(), Some(1024));
        assert_eq!(iter.next_delta(), Some(1024));
        assert_eq!(iter.next_delta(), Some(512));
        assert_eq!(iter.next_delta(), Some(512));
        assert_eq!(iter.next_delta(), None);
    }

    #[test]
    fn run_length_ctts_expansion() {
        let entries = vec![
            CttsEntry {
                sample_count: 2,
                sample_offset: 512,
            },
            CttsEntry {
                sample_count: 1,
                sample_offset: -256,
            },
        ];
        let mut iter = RunLengthIter::new(entries.into_iter());

        assert_eq!(iter.next_offset(), Some(512));
        assert_eq!(iter.next_offset(), Some(512));
        assert_eq!(iter.next_offset(), Some(-256));
        assert_eq!(iter.next_offset(), None);
    }

    // ── ChunkIter ───────────────────────────────────────────────────────

    #[test]
    fn chunk_iter_single_stsc_range() {
        let mut iter = ChunkIter {
            stsc: vec![StscEntry {
                first_chunk: 1,
                samples_per_chunk: 3,
                sample_description_index: 1,
            }]
            .into_iter()
            .peekable(),
            stco: vec![
                StcoEntry { chunk_offset: 1000 },
                StcoEntry { chunk_offset: 2000 },
            ]
            .into_iter(),
            current_chunk: 0,
            current: None,
        };

        let c1 = iter.next().unwrap();
        assert_eq!(
            (c1.offset, c1.sample_count, c1.description_index),
            (1000, 3, 1)
        );

        let c2 = iter.next().unwrap();
        assert_eq!(
            (c2.offset, c2.sample_count, c2.description_index),
            (2000, 3, 1)
        );

        assert!(iter.next().is_none());
    }

    #[test]
    fn chunk_iter_stsc_range_change() {
        let mut iter = ChunkIter {
            stsc: vec![
                StscEntry {
                    first_chunk: 1,
                    samples_per_chunk: 3,
                    sample_description_index: 1,
                },
                StscEntry {
                    first_chunk: 3,
                    samples_per_chunk: 2,
                    sample_description_index: 2,
                },
            ]
            .into_iter()
            .peekable(),
            stco: vec![
                StcoEntry { chunk_offset: 1000 },
                StcoEntry { chunk_offset: 2000 },
                StcoEntry { chunk_offset: 3000 },
            ]
            .into_iter(),
            current_chunk: 0,
            current: None,
        };

        let c1 = iter.next().unwrap();
        assert_eq!((c1.sample_count, c1.description_index), (3, 1));

        let c2 = iter.next().unwrap();
        assert_eq!((c2.sample_count, c2.description_index), (3, 1));

        let c3 = iter.next().unwrap();
        assert_eq!((c3.sample_count, c3.description_index), (2, 2));

        assert!(iter.next().is_none());
    }

    // ── StblSampleIter ──────────────────────────────────────────────────

    #[test]
    fn stbl_single_chunk() {
        // 3 samples in one chunk at offset 1000, uniform duration 1024
        let samples: Vec<_> = make_stbl_iter(
            vec![SttsEntry {
                sample_count: 3,
                sample_delta: 1024,
            }],
            None,
            vec![
                StszEntry { entry_size: 100 },
                StszEntry { entry_size: 200 },
                StszEntry { entry_size: 300 },
            ],
            vec![StscEntry {
                first_chunk: 1,
                samples_per_chunk: 3,
                sample_description_index: 1,
            }],
            vec![StcoEntry { chunk_offset: 1000 }],
            None,
        )
        .collect();

        assert_eq!(samples.len(), 3);

        assert_eq!(samples[0].offset, 1000);
        assert_eq!(samples[0].decode_time, 0);
        assert_eq!(samples[0].sample.size, 100);
        assert_eq!(samples[0].sample.duration, 1024);
        assert_eq!(samples[0].description_index, 1);
        assert!(samples[0].sample.is_sync);

        assert_eq!(samples[1].offset, 1100);
        assert_eq!(samples[1].decode_time, 1024);
        assert_eq!(samples[1].sample.size, 200);

        assert_eq!(samples[2].offset, 1300);
        assert_eq!(samples[2].decode_time, 2048);
        assert_eq!(samples[2].sample.size, 300);
    }

    #[test]
    fn stbl_multiple_chunks() {
        // Chunk 1 (offset 1000): 2 samples, desc=1
        // Chunk 2 (offset 5000): 1 sample, desc=2
        let samples: Vec<_> = make_stbl_iter(
            vec![SttsEntry {
                sample_count: 3,
                sample_delta: 512,
            }],
            None,
            vec![
                StszEntry { entry_size: 100 },
                StszEntry { entry_size: 200 },
                StszEntry { entry_size: 300 },
            ],
            vec![
                StscEntry {
                    first_chunk: 1,
                    samples_per_chunk: 2,
                    sample_description_index: 1,
                },
                StscEntry {
                    first_chunk: 2,
                    samples_per_chunk: 1,
                    sample_description_index: 2,
                },
            ],
            vec![
                StcoEntry { chunk_offset: 1000 },
                StcoEntry { chunk_offset: 5000 },
            ],
            None,
        )
        .collect();

        assert_eq!(samples.len(), 3);

        // Chunk 1
        assert_eq!(samples[0].offset, 1000);
        assert_eq!(samples[0].description_index, 1);
        assert_eq!(samples[1].offset, 1100);
        assert_eq!(samples[1].description_index, 1);

        // Chunk 2 — offset resets to chunk base
        assert_eq!(samples[2].offset, 5000);
        assert_eq!(samples[2].description_index, 2);
    }

    #[test]
    fn stbl_with_sync_samples() {
        // 4 samples; only 1 and 3 are sync (1-based in stss)
        let samples: Vec<_> = make_stbl_iter(
            vec![SttsEntry {
                sample_count: 4,
                sample_delta: 1024,
            }],
            None,
            vec![
                StszEntry { entry_size: 100 },
                StszEntry { entry_size: 100 },
                StszEntry { entry_size: 100 },
                StszEntry { entry_size: 100 },
            ],
            vec![StscEntry {
                first_chunk: 1,
                samples_per_chunk: 4,
                sample_description_index: 1,
            }],
            vec![StcoEntry { chunk_offset: 0 }],
            Some(vec![
                StssEntry { sample_number: 1 },
                StssEntry { sample_number: 3 },
            ]),
        )
        .collect();

        assert_eq!(samples.len(), 4);
        assert!(samples[0].sample.is_sync);
        assert!(!samples[1].sample.is_sync);
        assert!(samples[2].sample.is_sync);
        assert!(!samples[3].sample.is_sync);
    }

    #[test]
    fn stbl_with_ctts() {
        let samples: Vec<_> = make_stbl_iter(
            vec![SttsEntry {
                sample_count: 3,
                sample_delta: 1024,
            }],
            Some(vec![
                CttsEntry {
                    sample_count: 2,
                    sample_offset: 512,
                },
                CttsEntry {
                    sample_count: 1,
                    sample_offset: -256,
                },
            ]),
            vec![
                StszEntry { entry_size: 100 },
                StszEntry { entry_size: 100 },
                StszEntry { entry_size: 100 },
            ],
            vec![StscEntry {
                first_chunk: 1,
                samples_per_chunk: 3,
                sample_description_index: 1,
            }],
            vec![StcoEntry { chunk_offset: 0 }],
            None,
        )
        .collect();

        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].sample.composition_time_offset, 512);
        assert_eq!(samples[1].sample.composition_time_offset, 512);
        assert_eq!(samples[2].sample.composition_time_offset, -256);
    }

    #[test]
    fn stbl_without_ctts_defaults_to_zero() {
        let samples: Vec<_> = make_stbl_iter(
            vec![SttsEntry {
                sample_count: 1,
                sample_delta: 1024,
            }],
            None,
            vec![StszEntry { entry_size: 100 }],
            vec![StscEntry {
                first_chunk: 1,
                samples_per_chunk: 1,
                sample_description_index: 1,
            }],
            vec![StcoEntry { chunk_offset: 0 }],
            None,
        )
        .collect();

        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].sample.composition_time_offset, 0);
    }

    // ── TrafSampleIter ──────────────────────────────────────────────────

    #[test]
    fn traf_single_trun() {
        let samples: Vec<_> = make_traf_iter(
            vec![(
                TrunHeader {
                    data_offset: Some(100),
                    first_sample_flags: None,
                },
                vec![entry(1024, 500), entry(1024, 600)],
            )],
            0,
            0,
            None,
            1,
            1000,
            0,
        )
        .collect();

        assert_eq!(samples.len(), 2);

        // offset = base(1000) + data_offset(100) = 1100
        assert_eq!(samples[0].offset, 1100);
        assert_eq!(samples[0].sample.size, 500);
        assert_eq!(samples[0].sample.duration, 1024);
        assert_eq!(samples[0].decode_time, 0);
        assert_eq!(samples[0].description_index, 1);

        // offset = 1100 + 500 = 1600
        assert_eq!(samples[1].offset, 1600);
        assert_eq!(samples[1].decode_time, 1024);
    }

    #[test]
    fn traf_uses_defaults() {
        let no_fields = TrunEntry {
            duration: None,
            size: None,
            flags: None,
            composition_time_offset: None,
        };
        let samples: Vec<_> = make_traf_iter(
            vec![(
                TrunHeader {
                    data_offset: Some(0),
                    first_sample_flags: None,
                },
                vec![no_fields, no_fields],
            )],
            1024,
            256,
            Some(SampleFlags::non_sync()),
            1,
            0,
            0,
        )
        .collect();

        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].sample.duration, 1024);
        assert_eq!(samples[0].sample.size, 256);
        assert!(!samples[0].sample.is_sync);

        assert_eq!(samples[1].offset, 256);
    }

    #[test]
    fn traf_multiple_truns_with_data_offset() {
        let samples: Vec<_> = make_traf_iter(
            vec![
                (
                    TrunHeader {
                        data_offset: Some(100),
                        first_sample_flags: None,
                    },
                    vec![entry(1024, 500)],
                ),
                (
                    TrunHeader {
                        data_offset: Some(2000),
                        first_sample_flags: None,
                    },
                    vec![entry(512, 300)],
                ),
            ],
            0,
            0,
            None,
            1,
            0,
            0,
        )
        .collect();

        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].offset, 100); // base(0) + 100
        assert_eq!(samples[1].offset, 2000); // base(0) + 2000
        assert_eq!(samples[1].decode_time, 1024);
    }

    #[test]
    fn traf_sequential_truns_without_data_offset() {
        let samples: Vec<_> = make_traf_iter(
            vec![
                (
                    TrunHeader {
                        data_offset: Some(100),
                        first_sample_flags: None,
                    },
                    vec![entry(1024, 500)],
                ),
                (
                    TrunHeader {
                        data_offset: None,
                        first_sample_flags: None,
                    },
                    vec![entry(512, 300)],
                ),
            ],
            0,
            0,
            None,
            1,
            0,
            0,
        )
        .collect();

        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].offset, 100);
        assert_eq!(samples[1].offset, 600); // continues: 100 + 500
    }

    #[test]
    fn traf_first_sample_flags_override() {
        let samples: Vec<_> = make_traf_iter(
            vec![(
                TrunHeader {
                    data_offset: Some(0),
                    first_sample_flags: Some(SampleFlags::sync()),
                },
                vec![
                    TrunEntry {
                        duration: Some(1024),
                        size: Some(100),
                        flags: Some(SampleFlags::non_sync()),
                        composition_time_offset: None,
                    },
                    TrunEntry {
                        duration: Some(1024),
                        size: Some(100),
                        flags: Some(SampleFlags::non_sync()),
                        composition_time_offset: None,
                    },
                ],
            )],
            0,
            0,
            None,
            1,
            0,
            0,
        )
        .collect();

        assert_eq!(samples.len(), 2);
        assert!(samples[0].sample.is_sync); // first_sample_flags overrides
        assert!(!samples[1].sample.is_sync); // per-sample flags
    }

    #[test]
    fn traf_with_base_decode_time() {
        let samples: Vec<_> = make_traf_iter(
            vec![(
                TrunHeader {
                    data_offset: Some(0),
                    first_sample_flags: None,
                },
                vec![entry(1024, 100), entry(1024, 100)],
            )],
            0,
            0,
            None,
            1,
            0,
            10000,
        )
        .collect();

        assert_eq!(samples[0].decode_time, 10000);
        assert_eq!(samples[1].decode_time, 11024);
    }

    #[test]
    fn traf_empty_truns() {
        let samples: Vec<_> = make_traf_iter(vec![], 0, 0, None, 1, 0, 0).collect();
        assert!(samples.is_empty());
    }
}
