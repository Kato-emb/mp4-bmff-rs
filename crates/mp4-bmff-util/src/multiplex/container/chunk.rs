use mp4_bmff::boxes::bmff::*;

use crate::multiplex::Result;
use crate::multiplex::error::{Error, ErrorKind};

/// A struct representing the layout of chunks in a media track, including the sample-to-chunk mapping and chunk offsets. This struct provides methods for creating a chunk layout from given parameters, shifting chunk offsets by a specified delta, calculating the total number of samples based on the chunk layout, and resolving a sample index to its corresponding chunk and sample position within that chunk.
#[derive(Debug, Clone, Default)]
pub struct ChunkLayout {
    pub(crate) stsc_entries: Vec<StscEntry>,
    pub(crate) chunk_offsets: Vec<u64>,
}

impl ChunkLayout {
    /// Creates a new `ChunkLayout` from the given samples per chunk and chunk offsets. The `samples_per_chunk` slice specifies how many samples are in each chunk, and the `chunk_offsets` vector specifies the file offsets for each chunk. The number of entries in `samples_per_chunk` must match the number of entries in `chunk_offsets`, otherwise this function returns `None`.
    pub fn new(samples_per_chunk: &[u32], chunk_offsets: Vec<u64>) -> Option<Self> {
        if samples_per_chunk.len() != chunk_offsets.len() {
            return None;
        }

        let mut stsc_entries = Vec::new();
        let mut prev_spc: Option<u32> = None;

        for (i, &spc) in samples_per_chunk.iter().enumerate() {
            if prev_spc != Some(spc) {
                stsc_entries.push(StscEntry {
                    first_chunk: u32::try_from(i).ok()? + 1,
                    samples_per_chunk: spc,
                    sample_description_index: 1,
                });
                prev_spc = Some(spc);
            }
        }

        Some(ChunkLayout {
            stsc_entries,
            chunk_offsets,
        })
    }

    /// Concatenates this layout with another, returning a new layout whose chunks are `self`'s chunks followed by `other`'s chunks. The `stsc` entries of `other` are renumbered so their `first_chunk` indices continue after `self`'s chunks; an entry whose run is identical (same `samples_per_chunk` and `sample_description_index`) to the previous one is omitted to keep the run-length encoding compact.
    pub fn concat(&self, other: &ChunkLayout) -> ChunkLayout {
        let chunk_offset_shift = self.chunk_offsets.len() as u32;

        let mut stsc_entries = self.stsc_entries.clone();
        for entry in other.stsc_entries.iter() {
            let renumbered = StscEntry {
                first_chunk: entry.first_chunk + chunk_offset_shift,
                samples_per_chunk: entry.samples_per_chunk,
                sample_description_index: entry.sample_description_index,
            };
            if let Some(last) = stsc_entries.last() {
                if last.samples_per_chunk == renumbered.samples_per_chunk
                    && last.sample_description_index == renumbered.sample_description_index
                {
                    continue;
                }
            }
            stsc_entries.push(renumbered);
        }

        let mut chunk_offsets = self.chunk_offsets.clone();
        chunk_offsets.extend_from_slice(&other.chunk_offsets);

        ChunkLayout {
            stsc_entries,
            chunk_offsets,
        }
    }

    /// Returns the chunk layout shifted by the given delta. This method creates a new `ChunkLayout` with the same sample-to-chunk mapping but with all chunk offsets shifted by the specified delta value. Returns an error if any resulting offset would overflow or underflow `u64`.
    pub fn shift_offsets(&self, delta: i64) -> Result<ChunkLayout> {
        let chunk_offsets = self
            .chunk_offsets
            .iter()
            .map(|&o| {
                o.checked_add_signed(delta).ok_or_else(|| {
                    Error::new(ErrorKind::Overflow)
                        .with_message("Chunk offset overflow when shifting")
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ChunkLayout {
            stsc_entries: self.stsc_entries.clone(),
            chunk_offsets,
        })
    }

    /// Returns the total number of samples based on the chunk layout. This method calculates the total sample count by iterating through the sample-to-chunk mapping defined in `stsc_entries` and summing the number of samples in each chunk according to the specified `samples_per_chunk` values. The total sample count is returned as a `u32`.
    pub fn sample_count(&self) -> u32 {
        let total_chunks = self.chunk_offsets.len() as u32;
        let mut count: u32 = 0;

        for (i, entry) in self.stsc_entries.iter().enumerate() {
            let first_0 = entry.first_chunk - 1;
            let next_0 = self
                .stsc_entries
                .get(i + 1)
                .map(|e| e.first_chunk - 1)
                .unwrap_or(total_chunks);
            count += (next_0 - first_0) * entry.samples_per_chunk;
        }

        count
    }

    /// Resolves a sample index to its corresponding chunk and sample position within that chunk. This method iterates through the sample-to-chunk mapping defined in `stsc_entries` and calculates the chunk and sample position for the given sample index. Returns `None` if the sample index is out of bounds.
    pub(crate) fn resolve(&self, sample_index: u32) -> Option<(u32, u32)> {
        let total_chunks = self.chunk_offsets.len() as u32;
        let mut sample_cursor: u32 = 0;

        for (i, entry) in self.stsc_entries.iter().enumerate() {
            let first_0 = entry.first_chunk - 1;
            let next_0 = self
                .stsc_entries
                .get(i + 1)
                .map(|e| e.first_chunk - 1)
                .unwrap_or(total_chunks);

            let chunk_count = next_0 - first_0;
            let samples_in_run = chunk_count * entry.samples_per_chunk;

            if sample_index < sample_cursor + samples_in_run {
                let offset_in_run = sample_index - sample_cursor;
                let chunk_in_run = offset_in_run / entry.samples_per_chunk;
                let sample_in_chunk = offset_in_run % entry.samples_per_chunk;
                return Some((first_0 + chunk_in_run, sample_in_chunk));
            }

            sample_cursor += samples_in_run;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiplex::error::ErrorKind;

    fn stsc(first_chunk: u32, samples_per_chunk: u32) -> StscEntry {
        StscEntry {
            first_chunk,
            samples_per_chunk,
            sample_description_index: 1,
        }
    }

    #[test]
    fn new_rejects_length_mismatch() {
        assert!(ChunkLayout::new(&[1, 2], vec![100]).is_none());
        assert!(ChunkLayout::new(&[1], vec![100, 200]).is_none());
    }

    #[test]
    fn new_groups_consecutive_equal_samples_per_chunk() {
        let layout = ChunkLayout::new(&[2, 2, 2, 3, 3, 1], vec![10, 20, 30, 40, 50, 60]).unwrap();
        // expect grouping: [first=1,spc=2], [first=4,spc=3], [first=6,spc=1]
        assert_eq!(layout.stsc_entries.len(), 3);
        assert_eq!(layout.stsc_entries[0].first_chunk, 1);
        assert_eq!(layout.stsc_entries[0].samples_per_chunk, 2);
        assert_eq!(layout.stsc_entries[1].first_chunk, 4);
        assert_eq!(layout.stsc_entries[1].samples_per_chunk, 3);
        assert_eq!(layout.stsc_entries[2].first_chunk, 6);
        assert_eq!(layout.stsc_entries[2].samples_per_chunk, 1);
        assert_eq!(layout.chunk_offsets, vec![10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn concat_renumbers_stsc_first_chunk() {
        let a = ChunkLayout {
            stsc_entries: vec![stsc(1, 2)],
            chunk_offsets: vec![100, 200],
        };
        let b = ChunkLayout {
            stsc_entries: vec![stsc(1, 3)],
            chunk_offsets: vec![300, 400],
        };
        let c = a.concat(&b);
        assert_eq!(c.chunk_offsets, vec![100, 200, 300, 400]);
        assert_eq!(c.stsc_entries.len(), 2);
        assert_eq!(c.stsc_entries[0].first_chunk, 1);
        assert_eq!(c.stsc_entries[0].samples_per_chunk, 2);
        assert_eq!(c.stsc_entries[1].first_chunk, 3);
        assert_eq!(c.stsc_entries[1].samples_per_chunk, 3);
    }

    #[test]
    fn concat_merges_adjacent_identical_runs() {
        let a = ChunkLayout {
            stsc_entries: vec![stsc(1, 2)],
            chunk_offsets: vec![100, 200],
        };
        let b = ChunkLayout {
            stsc_entries: vec![stsc(1, 2)],
            chunk_offsets: vec![300],
        };
        let c = a.concat(&b);
        assert_eq!(c.chunk_offsets, vec![100, 200, 300]);
        // The duplicate stsc entry from `b` should be merged into `a`'s tail
        assert_eq!(c.stsc_entries.len(), 1);
        assert_eq!(c.stsc_entries[0].samples_per_chunk, 2);
        assert_eq!(c.sample_count(), 6);
    }

    #[test]
    fn shift_offsets_positive() {
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 1)],
            chunk_offsets: vec![100, 200, 300],
        };
        let shifted = layout.shift_offsets(50).unwrap();
        assert_eq!(shifted.chunk_offsets, vec![150, 250, 350]);
    }

    #[test]
    fn shift_offsets_negative() {
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 1)],
            chunk_offsets: vec![100, 200, 300],
        };
        let shifted = layout.shift_offsets(-50).unwrap();
        assert_eq!(shifted.chunk_offsets, vec![50, 150, 250]);
    }

    #[test]
    fn shift_offsets_underflow_errors() {
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 1)],
            chunk_offsets: vec![10],
        };
        let err = layout.shift_offsets(-100).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Overflow);
    }

    #[test]
    fn shift_offsets_overflow_errors() {
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 1)],
            chunk_offsets: vec![u64::MAX - 1],
        };
        assert_eq!(
            layout.shift_offsets(10).unwrap_err().kind(),
            ErrorKind::Overflow
        );
    }

    #[test]
    fn sample_count_simple() {
        // 3 chunks * 2 samples each = 6
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 2)],
            chunk_offsets: vec![10, 20, 30],
        };
        assert_eq!(layout.sample_count(), 6);
    }

    #[test]
    fn sample_count_with_run_change() {
        // 5 chunks: chunks 1..=3 carry 2 samples each (= 6), chunks 4..=5 carry 3 samples each (= 6), total 12
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 2), stsc(4, 3)],
            chunk_offsets: vec![10, 20, 30, 40, 50],
        };
        assert_eq!(layout.sample_count(), 3 * 2 + 2 * 3);
    }

    #[test]
    fn resolve_first_sample() {
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 2)],
            chunk_offsets: vec![10, 20, 30],
        };
        // sample 0 → chunk 0, position 0
        assert_eq!(layout.resolve(0), Some((0, 0)));
        // sample 1 → chunk 0, position 1
        assert_eq!(layout.resolve(1), Some((0, 1)));
        // sample 2 → chunk 1, position 0
        assert_eq!(layout.resolve(2), Some((1, 0)));
        // sample 5 → chunk 2, position 1 (last)
        assert_eq!(layout.resolve(5), Some((2, 1)));
    }

    #[test]
    fn resolve_out_of_range() {
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 2)],
            chunk_offsets: vec![10, 20, 30],
        };
        assert_eq!(layout.resolve(6), None);
    }

    #[test]
    fn resolve_across_run_change() {
        // chunks 0..2 (1..3): 2 spc → samples 0..3
        // chunk 2 (4): 3 spc → samples 4..6
        let layout = ChunkLayout {
            stsc_entries: vec![stsc(1, 2), stsc(3, 3)],
            chunk_offsets: vec![10, 20, 30],
        };
        assert_eq!(layout.resolve(3), Some((1, 1))); // last of first run
        assert_eq!(layout.resolve(4), Some((2, 0))); // first of second run
        assert_eq!(layout.resolve(6), Some((2, 2))); // last of second run
        assert_eq!(layout.resolve(7), None);
    }
}
