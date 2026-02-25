use mp4_bmff::boxes::bmff::{CttsEntry, SttsEntry};

use super::Sample;

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
