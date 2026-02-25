use mp4_bmff::boxes::bmff::{CttsEntry, SttsEntry};

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
