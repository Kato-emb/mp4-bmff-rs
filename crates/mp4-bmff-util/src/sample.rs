//! Utilities for working with MP4 sample tables (`stbl` and its child boxes).
//!
//! This module provides:
//! - `Sample`: A struct representing metadata for a single sample, including size, duration, composition time offset, sync sample status, and dependency flags.
//! - `Chunk`: A struct representing a chunk of samples, including its file offset and description index.
//! - `StblBuilder`: A builder for constructing `StblBox` instances from chunks of samples, automatically handling the logic for

use mp4_bmff::boxes::bmff::{
    ChunkOffset, Co64Box, Co64Entry, CttsBox, CttsEntry, SampleFlags, SampleSize, SdtpBox,
    SdtpEntry, StblBox, StcoBox, StcoEntry, StscBox, StscEntry, StsdBox, StssBox, StssEntry,
    StszBox, StszEntry, SttsBox, SttsEntry,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Metadata for a single sample
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sample {
    /// Byte size of the sample.
    pub size: u32,
    /// Duration in media timescale units.
    pub duration: u32,
    /// Composition time offset (CTS - DTS).
    pub composition_time_offset: i32,
    /// Whether this sample is a sync sample (random access point).
    pub is_sync: bool,
    /// Per-sample dependency flags from `sdtp`. `None` if `sdtp` was absent.
    pub dependency: Option<SampleFlags>,
}

/// A chunk of samples, along with its file offset and description index.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chunk {
    /// Sample description index (1-based).
    pub description_index: u32,
    /// File offset of the chunk's data.
    pub offset: u64,
    /// Samples in this chunk.
    pub samples: Vec<Sample>,
}

/// Builder for constructing sample tables from `stbl` child boxes.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct StblBuilder {
    inner: StblBox,
}

impl StblBuilder {
    /// Creates a new `StblBuilder` from the given `stsd` box.
    pub fn new(stsd: StsdBox) -> Self {
        StblBuilder {
            inner: StblBox {
                stsd,
                stts: SttsBox::default(),
                sample_size: SampleSize::Stsz(StszBox::default()),
                stsc: StscBox::default(),
                chunk_offset: ChunkOffset::Stco(StcoBox::default()),
                stdp: None,
                ctts: None,
                cslg: None,
                stss: Some(StssBox::default()), // Start with an empty stss; we can remove it later if all samples are sync samples.
                stsh: None,
                sdtp: None,
            },
        }
    }

    /// Adds a chunk of samples to the builder.
    pub fn add_chunk(&mut self, chunk: Chunk) {
        let sample_count = chunk.samples.len() as u32;

        // Update stsc with the new chunk's description index and sample count.
        if self.inner.stsc.entries.last().map_or(true, |e| {
            e.sample_description_index != chunk.description_index
                || e.samples_per_chunk != sample_count
        }) {
            let entry = StscEntry {
                first_chunk: (self.inner.chunk_offset.entries_count() + 1) as u32,
                samples_per_chunk: sample_count,
                sample_description_index: chunk.description_index,
            };
            self.inner.stsc.entries.push(entry);
        }

        // Update chunk offsets.
        match &mut self.inner.chunk_offset {
            ChunkOffset::Stco(stco) => {
                if let Ok(chunk_offset) = u32::try_from(chunk.offset) {
                    stco.entries.push(StcoEntry { chunk_offset });
                } else {
                    // If the offset exceeds 32 bits, we need to switch to Co64.
                    let mut co64 = Co64Box::default();
                    for entry in stco.entries.drain(..) {
                        let entry = Co64Entry {
                            chunk_offset: u64::from(entry.chunk_offset),
                        };
                        co64.entries.push(entry);
                    }
                    co64.entries.push(Co64Entry {
                        chunk_offset: chunk.offset,
                    });
                    self.inner.chunk_offset = ChunkOffset::Co64(co64);
                }
            }
            ChunkOffset::Co64(co64) => {
                let entry = Co64Entry {
                    chunk_offset: chunk.offset,
                };
                co64.entries.push(entry);
            }
        }

        for sample in chunk.samples {
            // Update stts with the sample's duration.
            if let Some(last) = self
                .inner
                .stts
                .entries
                .last_mut()
                .filter(|e| e.sample_delta == sample.duration)
            {
                last.sample_count += 1;
            } else {
                let entry = SttsEntry {
                    sample_count: 1,
                    sample_delta: sample.duration,
                };
                self.inner.stts.entries.push(entry);
            }

            // Update stsz with the sample's size.
            match &mut self.inner.sample_size {
                SampleSize::Stsz(stsz) => {
                    let entry = StszEntry {
                        entry_size: sample.size,
                    };
                    stsz.entries.push(entry);
                }
                _ => unreachable!(),
            }

            // Update stss if this is a sync sample.
            if sample.is_sync {
                let stss = self.inner.stss.get_or_insert_default();
                let entry = StssEntry {
                    sample_number: self.inner.sample_size.entries_count() as u32,
                };
                stss.entries.push(entry);
            }

            // Update ctts with the sample's composition time offset if it's non-zero.
            if sample.composition_time_offset != 0 || self.inner.ctts.is_some() {
                let ctts = self.inner.ctts.get_or_insert_with(|| {
                    let mut ctts = CttsBox::default();
                    let prev_count = (self.inner.sample_size.entries_count() - 1) as u32;
                    if prev_count > 0 {
                        let entry = CttsEntry {
                            sample_count: prev_count,
                            sample_offset: 0,
                        };
                        ctts.entries.push(entry);
                    }

                    ctts
                });

                if let Some(last) = ctts
                    .entries
                    .last_mut()
                    .filter(|e| e.sample_offset == sample.composition_time_offset)
                {
                    last.sample_count += 1;
                } else {
                    let entry = CttsEntry {
                        sample_count: 1,
                        sample_offset: sample.composition_time_offset,
                    };
                    ctts.entries.push(entry);
                }
            }

            // Update sdtp with the sample's dependency flags if present.
            if sample.dependency.is_some() || self.inner.sdtp.is_some() {
                let sdtp = self.inner.sdtp.get_or_insert_with(|| {
                    let mut sdtp = SdtpBox::default();
                    let prev_count = (self.inner.sample_size.entries_count() - 1) as u32;
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
        }
    }

    /// Finalizes the builder and returns the constructed `StblBox`.
    pub fn build(mut self) -> StblBox {
        if self
            .inner
            .stss
            .as_ref()
            .is_some_and(|stss| stss.entries.len() == self.inner.sample_size.entries_count())
        {
            // If every sample is a sync sample, we can omit the stss box.
            self.inner.stss = None;
        }

        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mp4_bmff::boxes::bmff::SampleDependsOn;

    fn sample(size: u32, duration: u32) -> Sample {
        Sample {
            size,
            duration,
            composition_time_offset: 0,
            is_sync: true,
            dependency: None,
        }
    }

    fn chunk(offset: u64, samples: Vec<Sample>) -> Chunk {
        Chunk {
            description_index: 1,
            offset,
            samples,
        }
    }

    fn stsd() -> StsdBox {
        StsdBox::default()
    }

    fn stsz_entries(stbl: &StblBox) -> &[StszEntry] {
        match &stbl.sample_size {
            SampleSize::Stsz(stsz) => &stsz.entries,
            _ => panic!("expected Stsz"),
        }
    }

    // ── basic single chunk ─────────────────────────────────────────────

    #[test]
    fn single_chunk() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(
            1000,
            vec![sample(100, 1024), sample(200, 1024), sample(300, 512)],
        ));
        let stbl = b.build();

        // stts: run-length encoded
        assert_eq!(stbl.stts.entries.len(), 2);
        assert_eq!(stbl.stts.entries[0].sample_count, 2);
        assert_eq!(stbl.stts.entries[0].sample_delta, 1024);
        assert_eq!(stbl.stts.entries[1].sample_count, 1);
        assert_eq!(stbl.stts.entries[1].sample_delta, 512);

        // stsz: one entry per sample
        let stsz = stsz_entries(&stbl);
        assert_eq!(stsz.len(), 3);
        assert_eq!(stsz[0].entry_size, 100);
        assert_eq!(stsz[1].entry_size, 200);
        assert_eq!(stsz[2].entry_size, 300);

        // stsc: single entry
        assert_eq!(stbl.stsc.entries.len(), 1);
        assert_eq!(stbl.stsc.entries[0].first_chunk, 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 3);
        assert_eq!(stbl.stsc.entries[0].sample_description_index, 1);

        // stco: one chunk
        match &stbl.chunk_offset {
            ChunkOffset::Stco(stco) => {
                assert_eq!(stco.entries.len(), 1);
                assert_eq!(stco.entries[0].chunk_offset, 1000);
            }
            _ => panic!("expected Stco"),
        }

        // all sync → stss omitted
        assert!(stbl.stss.is_none());

        // no ctts
        assert!(stbl.ctts.is_none());
    }

    // ── multiple chunks, same stsc run ─────────────────────────────────

    #[test]
    fn multiple_chunks_same_run() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(1000, vec![sample(100, 1024), sample(200, 1024)]));
        b.add_chunk(chunk(5000, vec![sample(300, 1024), sample(400, 1024)]));
        let stbl = b.build();

        // stsc: same samples_per_chunk and desc → single entry
        assert_eq!(stbl.stsc.entries.len(), 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 2);

        // stco: two chunks
        match &stbl.chunk_offset {
            ChunkOffset::Stco(stco) => {
                assert_eq!(stco.entries.len(), 2);
                assert_eq!(stco.entries[0].chunk_offset, 1000);
                assert_eq!(stco.entries[1].chunk_offset, 5000);
            }
            _ => panic!("expected Stco"),
        }

        // stts: all same duration → single entry
        assert_eq!(stbl.stts.entries.len(), 1);
        assert_eq!(stbl.stts.entries[0].sample_count, 4);
    }

    // ── stsc run changes ───────────────────────────────────────────────

    #[test]
    fn stsc_run_changes_on_sample_count() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(1000, vec![sample(100, 1024), sample(200, 1024)]));
        b.add_chunk(chunk(5000, vec![sample(300, 1024)])); // different sample count
        let stbl = b.build();

        assert_eq!(stbl.stsc.entries.len(), 2);
        assert_eq!(stbl.stsc.entries[0].first_chunk, 1);
        assert_eq!(stbl.stsc.entries[0].samples_per_chunk, 2);
        assert_eq!(stbl.stsc.entries[1].first_chunk, 2);
        assert_eq!(stbl.stsc.entries[1].samples_per_chunk, 1);
    }

    #[test]
    fn stsc_run_changes_on_description_index() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(1000, vec![sample(100, 1024)]));
        b.add_chunk(Chunk {
            description_index: 2,
            offset: 2000,
            samples: vec![sample(200, 1024)],
        });
        let stbl = b.build();

        assert_eq!(stbl.stsc.entries.len(), 2);
        assert_eq!(stbl.stsc.entries[0].sample_description_index, 1);
        assert_eq!(stbl.stsc.entries[1].sample_description_index, 2);
        assert_eq!(stbl.stsc.entries[1].first_chunk, 2);
    }

    // ── stss (sync samples) ───────────────────────────────────────────

    #[test]
    fn stss_partial_sync() {
        let sync = Sample {
            is_sync: true,
            ..sample(100, 1024)
        };
        let non_sync = Sample {
            is_sync: false,
            ..sample(100, 1024)
        };

        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![sync, non_sync, sync, non_sync]));
        let stbl = b.build();

        let stss = stbl.stss.as_ref().expect("stss should be present");
        assert_eq!(stss.entries.len(), 2);
        assert_eq!(stss.entries[0].sample_number, 1);
        assert_eq!(stss.entries[1].sample_number, 3);
    }

    #[test]
    fn stss_all_sync_omitted() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![sample(100, 1024), sample(200, 1024)]));
        let stbl = b.build();

        assert!(stbl.stss.is_none());
    }

    #[test]
    fn stss_no_sync_kept_empty() {
        let non_sync = Sample {
            is_sync: false,
            ..sample(100, 1024)
        };

        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![non_sync, non_sync, non_sync]));
        let stbl = b.build();

        let stss = stbl
            .stss
            .as_ref()
            .expect("stss should be present when no sync samples");
        assert_eq!(stss.entries.len(), 0);
    }

    // ── chunk offset: stco → co64 upgrade ─────────────────────────────

    #[test]
    fn chunk_offset_stays_stco_for_small_offsets() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(1000, vec![sample(100, 1024)]));
        b.add_chunk(chunk(u32::MAX as u64, vec![sample(100, 1024)]));
        let stbl = b.build();

        assert!(matches!(stbl.chunk_offset, ChunkOffset::Stco(_)));
    }

    #[test]
    fn chunk_offset_upgrades_to_co64() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(1000, vec![sample(100, 1024)]));
        b.add_chunk(chunk(u32::MAX as u64 + 1, vec![sample(200, 1024)]));
        let stbl = b.build();

        match &stbl.chunk_offset {
            ChunkOffset::Co64(co64) => {
                assert_eq!(co64.entries.len(), 2);
                assert_eq!(co64.entries[0].chunk_offset, 1000);
                assert_eq!(co64.entries[1].chunk_offset, u32::MAX as u64 + 1);
            }
            _ => panic!("expected Co64 after large offset"),
        }
    }

    #[test]
    fn chunk_offset_stays_co64_after_upgrade() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(u32::MAX as u64 + 1, vec![sample(100, 1024)]));
        // After upgrade, even small offsets go to Co64
        b.add_chunk(chunk(500, vec![sample(100, 1024)]));
        let stbl = b.build();

        match &stbl.chunk_offset {
            ChunkOffset::Co64(co64) => {
                assert_eq!(co64.entries.len(), 2);
                assert_eq!(co64.entries[1].chunk_offset, 500);
            }
            _ => panic!("expected Co64"),
        }
    }

    // ── ctts ───────────────────────────────────────────────────────────

    #[test]
    fn ctts_all_zero_omitted() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![sample(100, 1024)]));
        let stbl = b.build();

        assert!(stbl.ctts.is_none());
    }

    #[test]
    fn ctts_nonzero_from_start() {
        let s = |offset| Sample {
            composition_time_offset: offset,
            ..sample(100, 1024)
        };

        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![s(512), s(512), s(256)]));
        let stbl = b.build();

        let ctts = stbl.ctts.as_ref().expect("ctts should be present");
        assert_eq!(ctts.entries.len(), 2);
        assert_eq!(ctts.entries[0].sample_count, 2);
        assert_eq!(ctts.entries[0].sample_offset, 512);
        assert_eq!(ctts.entries[1].sample_count, 1);
        assert_eq!(ctts.entries[1].sample_offset, 256);

        // Encoding auto-determines version=0 for non-negative offsets.
        use mp4_bmff::BoxEncode;
        let mut buf = vec![0u8; ctts.encoded_len()];
        ctts.encode_into(&mut buf).unwrap();
        assert_eq!(buf[0], 0); // version byte
    }

    #[test]
    fn ctts_backfill_when_nonzero_appears_later() {
        let zero = sample(100, 1024);
        let nonzero = Sample {
            composition_time_offset: 512,
            ..sample(100, 1024)
        };

        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![zero, zero, nonzero]));
        let stbl = b.build();

        let ctts = stbl.ctts.as_ref().expect("ctts should be present");
        assert_eq!(ctts.entries.len(), 2);
        assert_eq!(ctts.entries[0].sample_count, 2);
        assert_eq!(ctts.entries[0].sample_offset, 0);
        assert_eq!(ctts.entries[1].sample_count, 1);
        assert_eq!(ctts.entries[1].sample_offset, 512);
    }

    #[test]
    fn ctts_negative_offset() {
        let s = Sample {
            composition_time_offset: -256,
            ..sample(100, 1024)
        };

        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![s]));
        let stbl = b.build();

        let ctts = stbl.ctts.as_ref().unwrap();
        assert_eq!(ctts.entries[0].sample_offset, -256);

        // Encoding auto-determines version=1 for negative offsets.
        use mp4_bmff::BoxEncode;
        let mut buf = vec![0u8; ctts.encoded_len()];
        ctts.encode_into(&mut buf).unwrap();
        assert_eq!(buf[0], 1); // version byte
    }

    // ── sdtp ───────────────────────────────────────────────────────────

    #[test]
    fn sdtp_absent_when_no_dependency() {
        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![sample(100, 1024)]));
        let stbl = b.build();

        assert!(stbl.sdtp.is_none());
    }

    #[test]
    fn sdtp_backfill_when_dependency_appears_later() {
        let plain = sample(100, 1024);
        let with_dep = Sample {
            dependency: Some(
                SampleFlags::non_sync().with_sample_depends_on(SampleDependsOn::Others),
            ),
            is_sync: false,
            ..sample(100, 1024)
        };

        let mut b = StblBuilder::new(stsd());
        b.add_chunk(chunk(0, vec![plain, plain, with_dep]));
        let stbl = b.build();

        let sdtp = stbl.sdtp.as_ref().expect("sdtp should be present");
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

    // ── empty builder ──────────────────────────────────────────────────

    #[test]
    fn build_empty() {
        let b = StblBuilder::new(stsd());
        let stbl = b.build();

        assert_eq!(stbl.stts.entries.len(), 0);
        assert_eq!(stsz_entries(&stbl).len(), 0);
        assert_eq!(stbl.stsc.entries.len(), 0);
        assert_eq!(stbl.chunk_offset.entries_count(), 0);
        assert!(stbl.stss.is_none()); // 0 == 0 → removed
        assert!(stbl.ctts.is_none());
        assert!(stbl.sdtp.is_none());
    }
}
