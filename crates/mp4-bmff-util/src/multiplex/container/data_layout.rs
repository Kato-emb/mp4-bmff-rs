use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::StscEntry;

#[derive(Debug, Clone)]
pub struct DataLayout {
    pub(crate) sample_sizes: Vec<u32>,
    pub(crate) stsc_entries: Vec<StscEntry>,
    pub(crate) chunk_offsets: Vec<u64>,
}

impl DataLayout {
    pub(crate) fn shift_offsets(&mut self, delta: i64) {
        for offset in self.chunk_offsets.iter_mut() {
            if delta.is_negative() {
                *offset = offset.saturating_sub(delta.wrapping_abs() as u64);
            } else {
                *offset = offset.saturating_add(delta as u64);
            }
        }
    }
}
