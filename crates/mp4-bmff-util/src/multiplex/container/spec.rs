use core::ops::Range;

use mp4_bmff::boxes::bmff::{CttsEntry, SampleFlags, SdtpEntry, StssEntry, StszEntry, SttsEntry};

/// A struct representing the sample specification for a track, including the sample timing, size, composition time offsets, sync sample indices, and sample flags. This struct is used to describe the samples in a track and to generate the corresponding BMFF boxes (e.g., `stts`, `stsz`, `ctts`, `stss`, `sdtp`) when composing a movie.
#[derive(Debug, Clone, Default)]
pub struct SampleSpec {
    pub(crate) stts_entries: Vec<SttsEntry>,
    pub(crate) stsz_entries: Vec<StszEntry>,
    pub(crate) ctts_entries: Option<Vec<CttsEntry>>,
    pub(crate) stss_entries: Option<Vec<StssEntry>>,
    pub(crate) sdtp_entries: Option<Vec<SdtpEntry>>,
}

impl SampleSpec {
    /// Returns the total number of samples described by this sample specification.
    pub fn sample_count(&self) -> u32 {
        self.stsz_entries.len() as u32
    }

    /// Returns the run-length-encoded sample timing entries.
    pub fn stts_entries(&self) -> &[SttsEntry] {
        &self.stts_entries
    }

    /// Returns the per-sample size entries.
    pub fn stsz_entries(&self) -> &[StszEntry] {
        &self.stsz_entries
    }

    /// Returns the run-length-encoded composition time offset entries, if present.
    pub fn ctts_entries(&self) -> Option<&[CttsEntry]> {
        self.ctts_entries.as_deref()
    }

    /// Returns the sync sample (key frame) entries, if present. When `None`, every sample is a sync sample.
    pub fn stss_entries(&self) -> Option<&[StssEntry]> {
        self.stss_entries.as_deref()
    }

    /// Returns the sample dependency entries, if present.
    pub fn sdtp_entries(&self) -> Option<&[SdtpEntry]> {
        self.sdtp_entries.as_deref()
    }

    /// Returns the total media duration of the samples described by this sample specification, in the timescale of the track. This is calculated as the sum of all sample deltas in the `stts` entries.
    pub fn media_duration(&self) -> u64 {
        self.stts_entries
            .iter()
            .map(|e| u64::from(e.sample_count) * u64::from(e.sample_delta))
            .sum()
    }

    /// Creates a `SampleSpec` from the given sample data. The `deltas` and `sizes` slices must have the same length, which represents the number of samples. The `offsets`, `sync_indices`, and `sample_flags` slices are optional and can be used to provide composition time offsets, sync sample indices, and sample flags, respectively.
    pub fn from_samples(
        deltas: &[u32],
        sizes: &[u32],
        offsets: Option<&[i32]>,
        sync_indices: Option<&[u32]>,
        sample_flags: Option<&[SampleFlags]>,
    ) -> Self {
        let stts_entries = Self::compress_stts(deltas);
        let stsz_entries = sizes.iter().map(|&s| StszEntry { entry_size: s }).collect();
        let ctts_entries = offsets
            .filter(|offsets| offsets.iter().any(|&o| o != 0))
            .map(Self::compress_ctts);
        let stss_entries = sync_indices.map(|indices| {
            indices
                .iter()
                .map(|&i| StssEntry {
                    sample_number: i + 1,
                })
                .collect()
        });

        let sdtp_entries = sample_flags.map(|flags| {
            flags
                .iter()
                .map(|&f| SdtpEntry {
                    is_leading: f.is_leading(),
                    sample_depends_on: f.sample_depends_on(),
                    sample_is_depended_on: f.sample_is_depended_on(),
                    sample_has_redundancy: f.sample_has_redundancy(),
                })
                .collect()
        });

        SampleSpec {
            stts_entries,
            stsz_entries,
            ctts_entries,
            stss_entries,
            sdtp_entries,
        }
    }

    /// Returns a new `SampleSpec` that describes a contiguous subset of the samples described by this `SampleSpec`, as specified by the given sample index range. The range is zero-based and half-open, meaning that `range.start` is the index of the first sample to include, and `range.end` is the index of the first sample to exclude. If the range is invalid (e.g., if `range.start >= range.end` or if `range.end` exceeds the total number of samples), then `None` is returned.
    pub fn slice(&self, range: Range<u32>) -> Option<SampleSpec> {
        if range.start >= range.end || range.end > self.sample_count() {
            return None;
        }

        Some(SampleSpec {
            stts_entries: Self::slice_stts(&self.stts_entries, range.clone())?,
            stsz_entries: self.stsz_entries[range.start as usize..range.end as usize].to_vec(),
            ctts_entries: self
                .ctts_entries
                .as_ref()
                .and_then(|ctts| Self::slice_ctts(ctts, range.clone())),
            stss_entries: self
                .stss_entries
                .as_ref()
                .map(|stss| Self::slice_stss(stss, range.clone())),
            sdtp_entries: self
                .sdtp_entries
                .as_ref()
                .map(|sdtp| sdtp[range.start as usize..range.end as usize].to_vec()),
        })
    }

    /// Returns a new `SampleSpec` that describes the concatenation of the samples described by this `SampleSpec` and another `SampleSpec`. The two `SampleSpec`s must have the same number of samples, and the sample descriptions in the second `SampleSpec` are assumed to follow immediately after the sample descriptions in this `SampleSpec`. If the two `SampleSpec`s have different numbers of samples, then the resulting `SampleSpec` will contain the concatenation of the sample descriptions, but the media duration and sync sample indices may not be correct.
    pub fn concat(&self, other: &SampleSpec) -> SampleSpec {
        let a_count = self.sample_count();

        SampleSpec {
            stts_entries: Self::concat_stts(&self.stts_entries, &other.stts_entries),
            stsz_entries: [self.stsz_entries.as_slice(), other.stsz_entries.as_slice()].concat(),
            ctts_entries: match (&self.ctts_entries, &other.ctts_entries) {
                (Some(a), Some(b)) => Some(Self::concat_ctts(a, b)),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(b.clone()),
                (None, None) => None,
            },
            stss_entries: match (&self.stss_entries, &other.stss_entries) {
                (Some(a), Some(b)) => Some(Self::concat_stss(a, b, a_count)),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(
                    b.iter()
                        .map(|e| StssEntry {
                            sample_number: e.sample_number + a_count,
                        })
                        .collect(),
                ),
                (None, None) => None,
            },
            sdtp_entries: match (&self.sdtp_entries, &other.sdtp_entries) {
                (Some(a), Some(b)) => Some([a.as_slice(), b.as_slice()].concat()),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(b.clone()),
                (None, None) => None,
            },
        }
    }

    fn compress_stts(deltas: &[u32]) -> Vec<SttsEntry> {
        let mut entries = Vec::new();
        for &d in deltas {
            if let Some(last) = entries.last_mut() {
                let last: &mut SttsEntry = last;
                if last.sample_delta == d {
                    last.sample_count += 1;
                    continue;
                }
            }

            entries.push(SttsEntry {
                sample_count: 1,
                sample_delta: d,
            });
        }

        entries
    }

    fn compress_ctts(offsets: &[i32]) -> Vec<CttsEntry> {
        let mut entries = Vec::new();
        for &o in offsets {
            if let Some(last) = entries.last_mut() {
                let last: &mut CttsEntry = last;
                if last.sample_offset == o {
                    last.sample_count += 1;
                    continue;
                }
            }
            entries.push(CttsEntry {
                sample_count: 1,
                sample_offset: o,
            });
        }

        entries
    }

    fn slice_stts(entries: &[SttsEntry], range: Range<u32>) -> Option<Vec<SttsEntry>> {
        let mut result = Vec::new();
        let mut cursor: u32 = 0;

        for entry in entries {
            let run_end = cursor + entry.sample_count;
            let overlap_start = range.start.max(cursor);
            let overlap_end = range.end.min(run_end);
            if overlap_start < overlap_end {
                result.push(SttsEntry {
                    sample_count: overlap_end - overlap_start,
                    sample_delta: entry.sample_delta,
                });
            }

            cursor = run_end;
            if cursor >= range.end {
                break;
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn slice_ctts(entries: &[CttsEntry], range: Range<u32>) -> Option<Vec<CttsEntry>> {
        let mut result = Vec::new();
        let mut cursor: u32 = 0;

        for entry in entries {
            let run_end = cursor + entry.sample_count;
            let overlap_start = range.start.max(cursor);
            let overlap_end = range.end.min(run_end);
            if overlap_start < overlap_end {
                result.push(CttsEntry {
                    sample_count: overlap_end - overlap_start,
                    sample_offset: entry.sample_offset,
                });
            }

            cursor = run_end;
            if cursor >= range.end {
                break;
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn slice_stss(entries: &[StssEntry], range: Range<u32>) -> Vec<StssEntry> {
        entries
            .iter()
            .filter_map(|e| {
                let zero_based = e.sample_number - 1;
                if zero_based >= range.start && zero_based < range.end {
                    Some(StssEntry {
                        sample_number: zero_based - range.start + 1,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn concat_stts(a: &[SttsEntry], b: &[SttsEntry]) -> Vec<SttsEntry> {
        let mut result = a.to_vec();
        for entry in b {
            if let Some(last) = result.last_mut() {
                if last.sample_delta == entry.sample_delta {
                    last.sample_count += entry.sample_count;
                    continue;
                }
            }

            result.push(entry.clone());
        }

        result
    }

    fn concat_ctts(a: &[CttsEntry], b: &[CttsEntry]) -> Vec<CttsEntry> {
        let mut result = a.to_vec();
        for entry in b {
            if let Some(last) = result.last_mut() {
                if last.sample_offset == entry.sample_offset {
                    last.sample_count += entry.sample_count;
                    continue;
                }
            }

            result.push(entry.clone());
        }

        result
    }

    fn concat_stss(a: &[StssEntry], b: &[StssEntry], a_count: u32) -> Vec<StssEntry> {
        let mut result = a.to_vec();
        result.extend(b.iter().map(|e| StssEntry {
            sample_number: e.sample_number + a_count,
        }));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SttsEntry / CttsEntry が PartialEq を実装していないため tuple へ射影して比較
    fn stts(e: &SttsEntry) -> (u32, u32) {
        (e.sample_count, e.sample_delta)
    }
    fn ctts(e: &CttsEntry) -> (u32, i32) {
        (e.sample_count, e.sample_offset)
    }

    #[test]
    fn from_samples_minimal() {
        let spec = SampleSpec::from_samples(&[100, 100, 100], &[10, 20, 30], None, None, None);
        assert_eq!(spec.sample_count(), 3);
        assert_eq!(spec.media_duration(), 300);
        // all deltas equal → single rle entry
        assert_eq!(spec.stts_entries().len(), 1);
        assert_eq!(spec.stts_entries()[0].sample_count, 3);
        assert_eq!(spec.stts_entries()[0].sample_delta, 100);
        assert!(spec.ctts_entries().is_none());
        assert!(spec.stss_entries().is_none());
        assert!(spec.sdtp_entries().is_none());
    }

    #[test]
    fn from_samples_varying_deltas_yield_multiple_entries() {
        let spec = SampleSpec::from_samples(&[100, 100, 200, 200, 100], &[1; 5], None, None, None);
        let entries: Vec<_> = spec.stts_entries().iter().map(stts).collect();
        assert_eq!(entries, vec![(2, 100), (2, 200), (1, 100)]);
    }

    #[test]
    fn from_samples_all_zero_offsets_omits_ctts() {
        let spec = SampleSpec::from_samples(&[100; 3], &[1; 3], Some(&[0, 0, 0]), None, None);
        assert!(
            spec.ctts_entries().is_none(),
            "all-zero offsets should be omitted"
        );
    }

    #[test]
    fn from_samples_nonzero_offset_emits_compressed_ctts() {
        let spec = SampleSpec::from_samples(&[100; 4], &[1; 4], Some(&[0, 33, 33, 0]), None, None);
        let entries: Vec<_> = spec.ctts_entries().unwrap().iter().map(ctts).collect();
        assert_eq!(entries, vec![(1, 0), (2, 33), (1, 0)]);
    }

    #[test]
    fn from_samples_sync_indices_become_one_based() {
        let spec = SampleSpec::from_samples(&[1; 5], &[1; 5], None, Some(&[0, 2, 4]), None);
        let stss = spec.stss_entries().unwrap();
        assert_eq!(
            stss.iter().map(|e| e.sample_number).collect::<Vec<_>>(),
            vec![1, 3, 5]
        );
    }

    #[test]
    fn slice_basic() {
        // 5 samples, all delta 100
        let spec = SampleSpec::from_samples(&[100; 5], &[10, 20, 30, 40, 50], None, None, None);
        let sliced = spec.slice(1..4).unwrap();
        assert_eq!(sliced.sample_count(), 3);
        assert_eq!(sliced.media_duration(), 300);
        assert_eq!(
            sliced
                .stsz_entries()
                .iter()
                .map(|e| e.entry_size)
                .collect::<Vec<_>>(),
            vec![20, 30, 40]
        );
    }

    #[test]
    fn slice_invalid_range_returns_none() {
        let spec = SampleSpec::from_samples(&[100; 3], &[1; 3], None, None, None);
        assert!(spec.slice(2..2).is_none());
        assert!(spec.slice(2..1).is_none());
        assert!(spec.slice(0..4).is_none());
    }

    #[test]
    fn slice_preserves_stts_run_boundaries() {
        // deltas: [100, 100, 200, 200, 100] → slice [1..4) = [100, 200, 200]
        let spec = SampleSpec::from_samples(&[100, 100, 200, 200, 100], &[1; 5], None, None, None);
        let sliced = spec.slice(1..4).unwrap();
        let entries: Vec<_> = sliced.stts_entries().iter().map(stts).collect();
        assert_eq!(entries, vec![(1, 100), (2, 200)]);
    }

    #[test]
    fn slice_stss_filters_and_rebases() {
        let spec = SampleSpec::from_samples(&[1; 6], &[1; 6], None, Some(&[0, 2, 4]), None);
        // sync indices in 0-based source: 0, 2, 4 → in slice [1..5) (0-based 1..5):
        //   keep 2 (→ rebased 2) and 4 (→ rebased 4). 0 is outside.
        let sliced = spec.slice(1..5).unwrap();
        let stss = sliced.stss_entries().unwrap();
        assert_eq!(
            stss.iter().map(|e| e.sample_number).collect::<Vec<_>>(),
            vec![2, 4]
        );
    }

    #[test]
    fn concat_merges_adjacent_stts() {
        let a = SampleSpec::from_samples(&[100, 100], &[1; 2], None, None, None);
        let b = SampleSpec::from_samples(&[100, 200], &[1; 2], None, None, None);
        let c = a.concat(&b);
        // [100,100] + [100,200] → [100×3, 200×1]
        let entries: Vec<_> = c.stts_entries().iter().map(stts).collect();
        assert_eq!(entries, vec![(3, 100), (1, 200)]);
        assert_eq!(c.sample_count(), 4);
    }

    #[test]
    fn concat_offsets_stss_indices_for_b_side() {
        let a = SampleSpec::from_samples(&[1; 3], &[1; 3], None, Some(&[0]), None);
        let b = SampleSpec::from_samples(&[1; 2], &[1; 2], None, Some(&[0]), None);
        let c = a.concat(&b);
        let stss = c.stss_entries().unwrap();
        // a has sync at 1 (1-based), b has sync at 1 → after concat shifted to (1, 4)
        assert_eq!(
            stss.iter().map(|e| e.sample_number).collect::<Vec<_>>(),
            vec![1, 4]
        );
    }

    #[test]
    fn compress_stts_single_value_runs() {
        let spec = SampleSpec::from_samples(&[42], &[1], None, None, None);
        assert_eq!(spec.stts_entries().len(), 1);
        assert_eq!(spec.stts_entries()[0].sample_count, 1);
        assert_eq!(spec.stts_entries()[0].sample_delta, 42);
    }

    #[test]
    fn media_duration_sums_runs() {
        let spec = SampleSpec::from_samples(&[100, 100, 200], &[1; 3], None, None, None);
        assert_eq!(spec.media_duration(), 100 + 100 + 200);
    }
}
