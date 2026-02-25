//! Sample table iteration utilities.
//!
//! Resolves per-sample metadata from `stbl` child boxes.

use mp4_bmff::boxes::bmff::{
    CttsEntry, SampleFlags, SdtpEntry, StblBoxView, StcoEntry, StscEntry, StssEntry, StszEntry,
    SttsEntry,
};
use mp4_bmff::error::Result;

/// Metadata for a single sample
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sample {
    /// Byte size of the sample.
    pub size: u64,
    /// Duration in media timescale units.
    pub duration: u32,
    /// Composition time offset (CTS - DTS).
    pub composition_time_offset: i32,
    /// Whether this sample is a sync sample (random access point).
    pub is_sync: bool,
    /// Per-sample dependency flags from `sdtp`. `None` if `sdtp` was absent.
    pub dependency: Option<SampleFlags>,
}

/// A sample with all metadata resolved, including the description index and file offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedSample {
    /// Sample metadata from the sample table.
    pub sample: Sample,
    /// Index into `stsd` (1-based).
    pub description_index: u32,
    /// Byte offset of the sample in the file.
    pub offset: u64,
    /// Decode time (DTS) in media timescale units.
    /// TODO: デコードタイミングは、サンプルが時間的にどの位置にあるかを表現するため必要。
    pub decode_time: u64,
}

// =============================================================================
// RunLengthIter — expands run-length entries one sample at a time
// =============================================================================

struct RunLengthIter<I: Iterator> {
    inner: I,
    remaining: u32,
    current: Option<I::Item>,
}

impl<I> RunLengthIter<I>
where
    I: Iterator,
    I::Item: Copy,
{
    fn new(iter: I) -> Self {
        Self {
            inner: iter,
            remaining: 0,
            current: None,
        }
    }
}

impl<I: Iterator<Item = SttsEntry>> RunLengthIter<I> {
    fn next_delta(&mut self) -> Option<u32> {
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
    fn next_offset(&mut self) -> Option<i32> {
        if self.remaining == 0 {
            let entry = self.inner.next()?;
            self.remaining = entry.sample_count;
            self.current = Some(entry);
        }
        self.remaining -= 1;
        self.current.map(|e| e.sample_offset)
    }
}

struct StscState<I: Iterator<Item = StscEntry>> {
    inner: core::iter::Peekable<I>,
    current_chunk: u32,
    remaining: u32,
    samples_per_chunk: u32,
    description_index: u32,
}

impl<I: Iterator<Item = StscEntry>> StscState<I> {
    fn new(iter: I) -> Self {
        Self {
            inner: iter.peekable(),
            current_chunk: 0,
            remaining: 0,
            samples_per_chunk: 0,
            description_index: 0,
        }
    }

    /// Returns `(description_index, new_chunk)` for the next sample.
    ///
    /// `new_chunk` is `true` when this sample is the first in a new chunk,
    /// signalling that the caller should advance the chunk offset iterator.
    fn next_sample(&mut self) -> (u32, bool) {
        if self.remaining == 0 {
            self.current_chunk += 1;

            if self
                .inner
                .peek()
                .is_some_and(|e| e.first_chunk <= self.current_chunk)
            {
                let entry = self.inner.next().unwrap();
                self.samples_per_chunk = entry.samples_per_chunk;
                self.description_index = entry.sample_description_index;
            }

            self.remaining = self.samples_per_chunk.saturating_sub(1);
            (self.description_index, true)
        } else {
            self.remaining -= 1;
            (self.description_index, false)
        }
    }
}

// =============================================================================
// SampleIterInner — generic over iterator types
// =============================================================================

struct SampleIterInner<
    Stts: Iterator,
    Ctts: Iterator,
    Stsc: Iterator<Item = StscEntry>,
    Stsz,
    Stco,
    Stss: Iterator,
    Sdtp,
> {
    stts: RunLengthIter<Stts>,
    ctts: Option<RunLengthIter<Ctts>>,
    stsc: StscState<Stsc>,
    stsz: Stsz,
    stco: Stco,
    stss: Option<core::iter::Peekable<Stss>>,
    sdtp: Option<Sdtp>,

    // cumulative state
    decode_time: u64,
    sample_number: u32,
    chunk_offset: u64,
    offset_in_chunk: u64,
}

impl<Stts, Ctts, Stsc, Stsz, Stco, Stss, Sdtp> Iterator
    for SampleIterInner<Stts, Ctts, Stsc, Stsz, Stco, Stss, Sdtp>
where
    Stts: Iterator<Item = SttsEntry>,
    Ctts: Iterator<Item = CttsEntry>,
    Stsc: Iterator<Item = StscEntry>,
    Stsz: Iterator<Item = StszEntry>,
    Stco: Iterator<Item = StcoEntry>,
    Stss: Iterator<Item = StssEntry>,
    Sdtp: Iterator<Item = SdtpEntry>,
{
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

// =============================================================================
// Builder functions
// =============================================================================

/// Creates a sample iterator from a [`StblBoxView`] (zero-copy).
pub fn samples_from_view<'a>(stbl: &StblBoxView<'a>) -> Result<impl Iterator<Item = Sample> + 'a> {
    let stts = RunLengthIter::new(stbl.stts()?.entries());
    let ctts = stbl.ctts()?.map(|c| RunLengthIter::new(c.entries()));
    let stsc = StscState::new(stbl.stsc()?.entries());
    let stsz = stbl.stsz()?.entries();
    let stco = stbl.stco()?.entries();
    let stss = stbl.stss()?.map(|s| s.entries().peekable());
    let sdtp = stbl.sdtp()?.map(|s| s.entries());

    Ok(SampleIterInner {
        stts,
        ctts,
        stsc,
        stsz,
        stco,
        stss,
        sdtp,
        decode_time: 0,
        sample_number: 0,
        chunk_offset: 0,
        offset_in_chunk: 0,
    })
}
