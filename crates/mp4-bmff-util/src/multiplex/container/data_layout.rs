use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::StscEntry;

#[derive(Debug, Clone, Default)]
pub struct DataLayout {
    pub(crate) stsc_entries: Vec<StscEntry>,
    pub(crate) chunk_offsets: Vec<u64>,
}

impl DataLayout {
    /// Returns the sample-to-chunk entries.
    pub fn stsc_entries(&self) -> &[StscEntry] {
        &self.stsc_entries
    }

    /// Returns the chunk offsets.
    pub fn chunk_offsets(&self) -> &[u64] {
        &self.chunk_offsets
    }
}
