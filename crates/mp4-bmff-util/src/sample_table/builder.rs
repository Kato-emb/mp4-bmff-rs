use mp4_bmff::boxes::bmff::{
    CttsBox, CttsEntry, SdtpBox, SdtpEntry, StblBox, StcoBox, StcoEntry, StscBox, StscEntry,
    StsdBox, StssEntry, StszBox, StszEntry, SttsBox, SttsEntry,
};

use super::Sample;

/// A builder for constructing a sample table (`stbl` box) from sample metadata.
#[derive(Debug)]
pub struct StblBuilder {
    inner: StblBox,
}

impl StblBuilder {
    /// Creates a new `StblBuilder` with the given `stsd` box.
    pub fn new(stsd: StsdBox) -> Self {
        let inner = StblBox {
            stsd,
            stdp: None,
            stts: SttsBox::default(),
            ctts: None,
            cslg: None,
            stss: None,
            stsh: None,
            sdtp: None,
            stsz: StszBox::default(),
            stsc: StscBox::default(),
            stco: StcoBox::default(),
        };

        Self { inner }
    }

    /// Adds a chunk of samples to the sample table.
    pub fn add_chunk(&mut self, samples: &[Sample], description_index: u32, offset: u64) {
        let sample_count = samples.len() as u32;
        // TODO: Handle 64-bit chunk offsets with `co64`
        let chunk_offset = offset as u32;

        // Update stsc
        if self.inner.stsc.entries.last().map_or(true, |e| {
            e.sample_description_index != description_index || e.samples_per_chunk != sample_count
        }) {
            self.inner.stsc.entries.push(StscEntry {
                first_chunk: (self.inner.stco.entries.len() as u32) + 1,
                samples_per_chunk: sample_count,
                sample_description_index: description_index,
            });
        }

        // Update stco
        self.inner.stco.entries.push(StcoEntry { chunk_offset });

        for sample in samples {
            // Update stts
            if let Some(last_entry) = self
                .inner
                .stts
                .entries
                .last_mut()
                .filter(|e| e.sample_delta == sample.duration)
            {
                last_entry.sample_count += 1;
            } else {
                self.inner.stts.entries.push(SttsEntry {
                    sample_count: 1,
                    sample_delta: sample.duration,
                });
            }

            // Update stsz
            self.inner.stsz.entries.push(StszEntry {
                entry_size: sample.size,
            });

            // Update stss
            if sample.is_sync {
                let stss = self.inner.stss.get_or_insert_default();
                stss.entries.push(StssEntry {
                    sample_number: (self.inner.stsz.entries.len() as u32),
                });
            }

            // Update ctts
            if sample.composition_time_offset != 0 || self.inner.ctts.is_some() {
                let ctts = self.inner.ctts.get_or_insert_with(|| {
                    let mut ctts = CttsBox::default();
                    let prev_count = (self.inner.stsz.entries.len() - 1) as u32;
                    if prev_count > 0 {
                        ctts.entries.push(CttsEntry {
                            sample_count: prev_count,
                            sample_offset: 0,
                        });
                    }
                    ctts
                });

                if let Some(last_entry) = ctts
                    .entries
                    .last_mut()
                    .filter(|e| e.sample_offset == sample.composition_time_offset)
                {
                    last_entry.sample_count += 1;
                } else {
                    if sample.composition_time_offset.is_negative() {
                        ctts.version = 1; // Use signed offsets if any sample has a negative offset
                    }

                    ctts.entries.push(CttsEntry {
                        sample_count: 1,
                        sample_offset: sample.composition_time_offset,
                    });
                }
            }

            // Update sdtp
            if sample.dependency.is_some() || self.inner.sdtp.is_some() {
                let sdtp = self.inner.sdtp.get_or_insert_with(|| {
                    let mut sdtp = SdtpBox::default();
                    let prev_count = (self.inner.stsz.entries.len() - 1) as u32;
                    for _ in 0..prev_count {
                        sdtp.entries.push(SdtpEntry::default());
                    }

                    sdtp
                });

                if let Some(dependency) = sample.dependency {
                    sdtp.entries.push(SdtpEntry {
                        is_leading: dependency.is_leading(),
                        sample_depends_on: dependency.sample_depends_on(),
                        sample_is_depended_on: dependency.sample_is_depended_on(),
                        sample_has_redundancy: dependency.sample_has_redundancy(),
                    });
                } else {
                    sdtp.entries.push(SdtpEntry::default());
                }
            }

            // TODO: Handle chunk offsets > 4GB with `co64`
        }
    }

    /// Finalizes the sample table and returns the constructed `StblBox`.
    pub fn build(mut self) -> StblBox {
        if self
            .inner
            .stss
            .as_ref()
            .is_some_and(|stss| stss.entries.len() == self.inner.stsz.entries.len())
        {
            // If every sample is a sync sample, we can omit the stss box
            self.inner.stss = None;
        }

        self.inner
    }
}

#[cfg(test)]
mod tests {
    use mp4_bmff::boxes::bmff::{SampleDependsOn, SampleFlags};

    use super::*;

    fn sample(size: u32, duration: u32) -> Sample {
        Sample {
            size,
            duration,
            composition_time_offset: 0,
            is_sync: true,
            dependency: None,
        }
    }

    fn stsd() -> StsdBox {
        StsdBox::default()
    }

    // ── basic single chunk ─────────────────────────────────────────────

    #[test]
    fn single_chunk() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(
            &[sample(100, 1024), sample(200, 1024), sample(300, 512)],
            1,
            1000,
        );
        let stbl = b.build();

        // stts: run-length encoded
        assert_eq!(stbl.stts.entries.len(), 2);
        assert_eq!(stbl.stts.entries[0].sample_count, 2);
        assert_eq!(stbl.stts.entries[0].sample_delta, 1024);
        assert_eq!(stbl.stts.entries[1].sample_count, 1);
        assert_eq!(stbl.stts.entries[1].sample_delta, 512);

        // stsz: one entry per sample
        assert_eq!(stbl.stsz.entries.len(), 3);
        assert_eq!(stbl.stsz.entries[0].entry_size, 100);
        assert_eq!(stbl.stsz.entries[1].entry_size, 200);
        assert_eq!(stbl.stsz.entries[2].entry_size, 300);

        // stsc: single entry
        assert_eq!(stbl.stsc.entries.len(), 1);
        assert_eq!(stbl.stsc.entries[0].first_chunk, 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 3);
        assert_eq!(stbl.stsc.entries[0].sample_description_index, 1);

        // stco: one chunk
        assert_eq!(stbl.stco.entries.len(), 1);
        assert_eq!(stbl.stco.entries[0].chunk_offset, 1000);

        // all sync → stss omitted
        assert!(stbl.stss.is_none());

        // no ctts
        assert!(stbl.ctts.is_none());
    }

    // ── multiple chunks, same stsc run ─────────────────────────────────

    #[test]
    fn multiple_chunks_same_run() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(&[sample(100, 1024), sample(200, 1024)], 1, 1000);
        b.add_chunk(&[sample(300, 1024), sample(400, 1024)], 1, 5000);
        let stbl = b.build();

        // stsc: same samples_per_chunk and desc → single entry
        assert_eq!(stbl.stsc.entries.len(), 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 2);

        // stco: two chunks
        assert_eq!(stbl.stco.entries.len(), 2);
        assert_eq!(stbl.stco.entries[0].chunk_offset, 1000);
        assert_eq!(stbl.stco.entries[1].chunk_offset, 5000);

        // stts: all same duration → single entry
        assert_eq!(stbl.stts.entries.len(), 1);
        assert_eq!(stbl.stts.entries[0].sample_count, 4);
    }

    // ── stsc run changes ───────────────────────────────────────────────

    #[test]
    fn stsc_run_changes_on_sample_count() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(&[sample(100, 1024), sample(200, 1024)], 1, 1000);
        b.add_chunk(&[sample(300, 1024)], 1, 5000); // different sample count
        let stbl = b.build();

        assert_eq!(stbl.stsc.entries.len(), 2);
        assert_eq!(stbl.stsc.entries[0].first_chunk, 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 2);
        assert_eq!(stbl.stsc.entries[0].sample_description_index, 1);
        assert_eq!(stbl.stsc.entries[1].first_chunk, 2);
        assert_eq!(stbl.stsc.entries[1].samples_per_chunk, 1);
        assert_eq!(stbl.stsc.entries[1].sample_description_index, 1);
    }

    #[test]
    fn stsc_run_changes_on_description_index() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(&[sample(100, 1024)], 1, 1000);
        b.add_chunk(&[sample(200, 1024)], 2, 2000); // different desc
        let stbl = b.build();

        assert_eq!(stbl.stsc.entries.len(), 2);
        assert_eq!(stbl.stsc.entries[0].sample_description_index, 1);
        assert_eq!(stbl.stsc.entries[1].sample_description_index, 2);
        assert_eq!(stbl.stsc.entries[1].first_chunk, 2);
    }

    // ── stss (sync samples) ───────────────────────────────────────────

    #[test]
    fn stss_partial_sync() {
        let mut b = StblBuilder::new(stsd());
        let sync = Sample {
            is_sync: true,
            ..sample(100, 1024)
        };
        let non_sync = Sample {
            is_sync: false,
            ..sample(100, 1024)
        };
        b.add_chunk(&[sync, non_sync, sync, non_sync], 1, 0);
        let stbl = b.build();

        let stss = stbl.stss.as_ref().expect("stss should be present");
        assert_eq!(stss.entries.len(), 2);
        assert_eq!(stss.entries[0].sample_number, 1);
        assert_eq!(stss.entries[1].sample_number, 3);
    }

    #[test]
    fn stss_all_sync_omitted() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(&[sample(100, 1024), sample(200, 1024)], 1, 0);
        let stbl = b.build();

        assert!(stbl.stss.is_none());
    }

    // ── ctts ───────────────────────────────────────────────────────────

    #[test]
    fn ctts_all_zero_omitted() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(&[sample(100, 1024)], 1, 0);
        let stbl = b.build();

        assert!(stbl.ctts.is_none());
    }

    #[test]
    fn ctts_nonzero_from_start() {
        let mut b = StblBuilder::new(stsd());
        let s1 = Sample {
            composition_time_offset: 512,
            ..sample(100, 1024)
        };
        let s2 = Sample {
            composition_time_offset: 512,
            ..sample(100, 1024)
        };
        let s3 = Sample {
            composition_time_offset: 256,
            ..sample(100, 1024)
        };
        b.add_chunk(&[s1, s2, s3], 1, 0);
        let stbl = b.build();

        let ctts = stbl.ctts.as_ref().expect("ctts should be present");
        assert_eq!(ctts.version, 0);
        assert_eq!(ctts.entries.len(), 2);
        assert_eq!(ctts.entries[0].sample_count, 2);
        assert_eq!(ctts.entries[0].sample_offset, 512);
        assert_eq!(ctts.entries[1].sample_count, 1);
        assert_eq!(ctts.entries[1].sample_offset, 256);
    }

    #[test]
    fn ctts_backfill_when_nonzero_appears_later() {
        let mut b = StblBuilder::new(stsd());
        let zero = sample(100, 1024);
        let nonzero = Sample {
            composition_time_offset: 512,
            ..sample(100, 1024)
        };
        b.add_chunk(&[zero, zero, nonzero], 1, 0);
        let stbl = b.build();

        let ctts = stbl.ctts.as_ref().expect("ctts should be present");
        assert_eq!(ctts.entries.len(), 2);
        // backfilled zeros for first 2 samples
        assert_eq!(ctts.entries[0].sample_count, 2);
        assert_eq!(ctts.entries[0].sample_offset, 0);
        assert_eq!(ctts.entries[1].sample_count, 1);
        assert_eq!(ctts.entries[1].sample_offset, 512);
    }

    #[test]
    fn ctts_negative_sets_version_1() {
        let mut b = StblBuilder::new(stsd());
        let s = Sample {
            composition_time_offset: -256,
            ..sample(100, 1024)
        };
        b.add_chunk(&[s], 1, 0);
        let stbl = b.build();

        let ctts = stbl.ctts.as_ref().unwrap();
        assert_eq!(ctts.version, 1);
        assert_eq!(ctts.entries[0].sample_offset, -256);
    }

    // ── sdtp ───────────────────────────────────────────────────────────

    #[test]
    fn sdtp_absent_when_no_dependency() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(&[sample(100, 1024)], 1, 0);
        let stbl = b.build();

        assert!(stbl.sdtp.is_none());
    }

    #[test]
    fn sdtp_backfill_when_dependency_appears_later() {
        let mut b = StblBuilder::new(stsd());
        let plain = sample(100, 1024);
        let with_dep = Sample {
            dependency: Some(
                SampleFlags::non_sync().with_sample_depends_on(SampleDependsOn::Others),
            ),
            is_sync: false,
            ..sample(100, 1024)
        };
        b.add_chunk(&[plain, plain, with_dep], 1, 0);
        let stbl = b.build();

        let sdtp = stbl.sdtp.as_ref().expect("sdtp should be present");
        // 2 backfilled defaults + 1 with dependency
        assert_eq!(sdtp.entries.len(), 3);
        assert_eq!(
            sdtp.entries[0].sample_depends_on,
            SampleDependsOn::default()
        );
        assert_eq!(
            sdtp.entries[1].sample_depends_on,
            SampleDependsOn::default()
        );
        assert_eq!(sdtp.entries[2].sample_depends_on, SampleDependsOn::Others);
    }

    #[test]
    fn sdtp_default_for_none_after_some() {
        let mut b = StblBuilder::new(stsd());
        let with_dep = Sample {
            dependency: Some(SampleFlags::sync()),
            ..sample(100, 1024)
        };
        let plain = sample(100, 1024);
        b.add_chunk(&[with_dep, plain], 1, 0);
        let stbl = b.build();

        let sdtp = stbl.sdtp.as_ref().unwrap();
        assert_eq!(sdtp.entries.len(), 2);
        assert_eq!(
            sdtp.entries[1].sample_depends_on,
            SampleDependsOn::default()
        );
    }

    // ── round-trip with iter ───────────────────────────────────────────

    #[test]
    fn round_trip_with_resolved_samples() {
        use crate::sample_table::SampleTableExt;

        let mut b = StblBuilder::new(stsd());
        let samples = [sample(100, 1024), sample(200, 1024), sample(300, 512)];
        b.add_chunk(&samples, 1, 1000);
        let stbl = b.build();

        let resolved: Vec<_> = stbl.resolved_samples().unwrap().collect();

        assert_eq!(resolved.len(), 3);

        assert_eq!(resolved[0].offset, 1000);
        assert_eq!(resolved[0].decode_time, 0);
        assert_eq!(resolved[0].sample.size, 100);
        assert_eq!(resolved[0].sample.duration, 1024);
        assert_eq!(resolved[0].description_index, 1);

        assert_eq!(resolved[1].offset, 1100);
        assert_eq!(resolved[1].decode_time, 1024);
        assert_eq!(resolved[1].sample.size, 200);

        assert_eq!(resolved[2].offset, 1300);
        assert_eq!(resolved[2].decode_time, 2048);
        assert_eq!(resolved[2].sample.size, 300);
    }

    #[test]
    fn round_trip_multiple_chunks() {
        use crate::sample_table::SampleTableExt;

        let mut b = StblBuilder::new(stsd());
        b.add_chunk(&[sample(100, 1024), sample(200, 1024)], 1, 1000);
        b.add_chunk(&[sample(300, 512)], 2, 5000);
        let stbl = b.build();

        let resolved: Vec<_> = stbl.resolved_samples().unwrap().collect();

        assert_eq!(resolved.len(), 3);

        // chunk 1
        assert_eq!(resolved[0].offset, 1000);
        assert_eq!(resolved[0].description_index, 1);
        assert_eq!(resolved[1].offset, 1100);
        assert_eq!(resolved[1].description_index, 1);

        // chunk 2
        assert_eq!(resolved[2].offset, 5000);
        assert_eq!(resolved[2].description_index, 2);
        assert_eq!(resolved[2].decode_time, 2048);
    }
}
