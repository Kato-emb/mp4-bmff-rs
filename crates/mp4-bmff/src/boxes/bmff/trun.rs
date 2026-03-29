//! Track Run Box (`trun`) implementation.
//!
//! The Track Run Box contains per-sample information for a contiguous run
//! of samples in a track fragment. Multiple `trun` boxes may appear in a
//! single track fragment, each describing a separate run of samples.
//!
//! This box resides within the Track Fragment Box (`traf`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use super::common::SampleFlags;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Track Run Box (`trun`).
    ///
    /// Control which optional fields are present at box and sample level.
    TrunFlags {
        /// Data offset field is present (signed 32-bit offset from base).
        DATA_OFFSET_PRESENT = 0x000001,
        /// First sample flags field is present (overrides sample\[0\] flags).
        FIRST_SAMPLE_FLAGS_PRESENT = 0x000004,
        /// Per-sample duration values are present.
        SAMPLE_DURATION_PRESENT = 0x000100,
        /// Per-sample size values are present.
        SAMPLE_SIZE_PRESENT = 0x000200,
        /// Per-sample flags values are present.
        SAMPLE_FLAGS_PRESENT = 0x000400,
        /// Per-sample composition time offsets are present.
        SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT = 0x000800,
    }
);

fn sample_data_size(flags: TrunFlags) -> usize {
    let mut size = 0;
    if flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT) {
        size += 4;
    }
    if flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT) {
        size += 4;
    }
    if flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT) {
        size += 4;
    }
    if flags.contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT) {
        size += 4;
    }
    size
}

/// Sample data from a Track Run Box (`trun`).
///
/// Contains per-sample properties. Fields are `Some` only when the
/// corresponding flag is set in the parent `trun` box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrunEntry {
    /// Sample duration in track timescale units.
    pub duration: Option<u32>,
    /// Sample size in bytes.
    pub size: Option<u32>,
    /// Sample flags (sync, dependency info, degradation priority).
    pub flags: Option<SampleFlags>,
    /// Composition time offset relative to decode time.
    ///
    /// Valid range depends on box version:
    /// - version 0: `0..=u32::MAX` (unsigned)
    /// - version 1: `i32::MIN..=i32::MAX` (signed)
    pub composition_time_offset: Option<i64>,
}

/// An iterator over samples in a Track Run Box (`trun`).
#[derive(Debug)]
struct TrunEntryIter<'a> {
    samples: &'a [u8],
    flags: TrunFlags,
    version: u8,
}

impl Iterator for TrunEntryIter<'_> {
    type Item = Result<TrunEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        // When samples slice is empty, return None immediately
        if self.samples.is_empty() {
            return None;
        }

        let sample_size = sample_data_size(self.flags);
        let chunk = &self.samples[..sample_size];
        self.samples = &self.samples[sample_size..];

        let mut cursor = ReadCursor::new(chunk);

        let duration = if self.flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT) {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let size = if self.flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT) {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let flags = if self.flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT) {
            match cursor.read_u32_be() {
                Ok(v) => Some(SampleFlags::from_raw(v)),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let composition_time_offset = if self
            .flags
            .contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT)
        {
            if self.version == 0 {
                match cursor.read_u32_be() {
                    Ok(v) => Some(i64::from(v)),
                    Err(e) => return Some(Err(e.into())),
                }
            } else {
                match cursor.read_i32_be() {
                    Ok(v) => Some(i64::from(v)),
                    Err(e) => return Some(Err(e.into())),
                }
            }
        } else {
            None
        };

        Some(Ok(TrunEntry {
            duration,
            size,
            flags,
            composition_time_offset,
        }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let sample_size = sample_data_size(self.flags);
        if sample_size == 0 {
            (0, Some(0))
        } else {
            let count = self.samples.len() / sample_size;
            (count, Some(count))
        }
    }
}

impl ExactSizeIterator for TrunEntryIter<'_> {}

/// A reference to a Track Run Box (`trun`).
///
/// Describes a contiguous run of samples with optional per-sample data.
/// Flags control which fields are present at both box and sample level.
///
/// # Structure
///
/// - `version`: Box version (0 or 1, affects composition time offset sign).
/// - `flags`: Indicate which optional fields are present.
/// - `sample_count`: Number of samples in this run.
/// - `data_offset`: Offset from base to first sample's data.
/// - `first_sample_flags`: Special flags for first sample only.
/// - Per-sample: duration, size, flags, composition_time_offset.
#[derive(Debug)]
pub struct TrunBoxView<'a> {
    /// Box version (0 or 1; affects composition time offset interpretation).
    pub version: u8,
    /// Flags indicating which fields are present.
    pub flags: TrunFlags,
    /// Number of samples described in this run.
    pub sample_count: u32,
    /// Signed offset from base to first sample's data in `mdat`.
    pub data_offset: Option<i32>,
    /// Flags for first sample (overrides per-sample flags if both present).
    pub first_sample_flags: Option<SampleFlags>,
    samples: &'a [u8],
}

impl<'a> TrunBoxView<'a> {
    /// Returns an iterator over the entries in the Track Run Box (`trun`).
    pub fn entries(&self) -> impl ExactSizeIterator<Item = Result<TrunEntry>> + 'a {
        TrunEntryIter {
            samples: self.samples,
            flags: self.flags,
            version: self.version,
        }
    }
}

impl BoxCodec for TrunBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::TRUN
    }
}

impl<'de> BoxDecode<'de> for TrunBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = TrunFlags::from_be_bytes(cur.read_array::<3>()?);

        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "trun version must be 0 or 1",
                    got: version,
                },
                BoxType::TRUN,
            ));
        }

        let sample_count = cur.read_u32_be()?;

        let data_offset = if flags.contains(TrunFlags::DATA_OFFSET_PRESENT) {
            Some(cur.read_i32_be()?)
        } else {
            None
        };

        let first_sample_flags = if flags.contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT) {
            Some(SampleFlags::from_raw(cur.read_u32_be()?))
        } else {
            None
        };

        let expected_size = sample_count as usize * sample_data_size(flags);

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "trun samples size does not match sample_count",
                    got: cur.remaining() as u64,
                },
                BoxType::TRUN,
            ));
        }

        let samples = cur.take(expected_size)?;

        Ok(TrunBoxView {
            version,
            flags,
            sample_count,
            data_offset,
            first_sample_flags,
            samples,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Track Run Box (`trun`).
    ///
    /// This is the owned variant of [`TrunBoxView`] that stores samples
    /// in a heap-allocated vector.
    ///
    /// # Structure
    ///
    /// - `version`: Box version (0 or 1).
    /// - `flags`: Indicate which per-sample fields are present.
    /// - `data_offset`: Offset to first sample's data.
    /// - `first_sample_flags`: Special flags for first sample.
    /// - `samples`: Per-sample duration, size, flags, and composition offset.
    #[derive(Debug, Clone)]
    pub struct TrunBox {
        /// Box version (0 or 1; affects composition time offset sign).
        version: u8,
        /// Flags indicating which fields are present.
        flags: TrunFlags,
        /// Number of samples in this run (tracked independently of per-sample fields
        /// so that sample_count remains correct even when all fields use defaults).
        sample_count: u32,
        /// Signed offset from base to first sample's data.
        data_offset: Option<i32>,
        /// Flags for first sample (overrides sample\[0\].flags if present).
        first_sample_flags: Option<SampleFlags>,
        // Per-sample data stored in separate vectors for each field, or None if not present.
        sample_durations: Option<Vec<u32>>,
        sample_sizes: Option<Vec<u32>>,
        sample_flags: Option<Vec<SampleFlags>>,
        sample_composition_time_offsets: Option<Vec<i64>>,
    }

    impl TrunBox {
        /// Creates a new `TrunBox` with the specified flags and signed composition time offset.
        /// The `signed_cto` parameter determines whether the box version is set to 1 (signed CTO) or 0 (unsigned CTO).
        pub fn new(flags: TrunFlags, signed_cto: bool) -> Self {
            let sample_durations = if flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT) {
                Some(Vec::new())
            } else {
                None
            };

            let sample_sizes = if flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT) {
                Some(Vec::new())
            } else {
                None
            };

            let sample_flags = if flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT) {
                Some(Vec::new())
            } else {
                None
            };

            let sample_composition_time_offsets =
                if flags.contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT) {
                    Some(Vec::new())
                } else {
                    None
                };

            TrunBox {
                version: if signed_cto { 1 } else { 0 },
                flags,
                sample_count: 0,
                data_offset: None,
                first_sample_flags: None,
                sample_durations,
                sample_sizes,
                sample_flags,
                sample_composition_time_offsets,
            }
        }

        /// Returns the version of this `trun` box.
        pub fn version(&self) -> u8 {
            self.version
        }

        /// Returns the flags indicating which fields are present in this `trun` box.
        pub fn flags(&self) -> TrunFlags {
            self.flags
        }

        /// Returns the data offset for this `trun` box, or `None` if the data offset field is not present.
        pub fn data_offset(&self) -> Option<i32> {
            self.data_offset
        }

        /// Sets the data offset for this `trun` box. This field is only valid if the `DATA_OFFSET_PRESENT` flag is set.
        pub fn set_data_offset(&mut self, offset: i32) {
            self.data_offset = Some(offset);
            self.flags.insert(TrunFlags::DATA_OFFSET_PRESENT);
        }

        /// Returns the first sample flags for this `trun` box, or `None` if the first sample flags field is not present.
        pub fn first_sample_flags(&self) -> Option<SampleFlags> {
            self.first_sample_flags
        }

        /// Sets the first sample flags for this `trun` box. This field is only valid if the `FIRST_SAMPLE_FLAGS_PRESENT` flag is set.
        pub fn set_first_sample_flags(&mut self, flags: SampleFlags) {
            self.first_sample_flags = Some(flags);
            self.flags.insert(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT);
        }

        /// Returns the number of samples described in this `trun` box.
        pub fn sample_count(&self) -> usize {
            self.sample_count as usize
        }

        /// Returns a reference to the sample durations vector, or `None` if the sample duration field is not present.
        pub fn sample_durations(&self) -> Option<&[u32]> {
            self.sample_durations.as_deref()
        }

        /// Returns a reference to the sample sizes vector, or `None` if the sample size field is not present.
        pub fn sample_sizes(&self) -> Option<&[u32]> {
            self.sample_sizes.as_deref()
        }

        /// Returns a reference to the sample flags vector, or `None` if the sample flags field is not present.
        pub fn sample_flags(&self) -> Option<&[SampleFlags]> {
            self.sample_flags.as_deref()
        }

        /// Returns a reference to the sample composition time offsets vector, or `None` if the sample composition time offsets field is not present.
        pub fn sample_composition_time_offsets(&self) -> Option<&[i64]> {
            self.sample_composition_time_offsets.as_deref()
        }

        /// Returns the `TrunEntry` for the sample at the given index, or `None`
        /// if the index is out of bounds.
        pub fn get_entry(&self, index: usize) -> Option<TrunEntry> {
            if index >= self.sample_count() {
                return None;
            }

            let duration = self
                .sample_durations
                .as_ref()
                .and_then(|v| v.get(index))
                .copied();
            let size = self
                .sample_sizes
                .as_ref()
                .and_then(|v| v.get(index))
                .copied();
            let flags = self
                .sample_flags
                .as_ref()
                .and_then(|v| v.get(index))
                .copied();
            let composition_time_offset = self
                .sample_composition_time_offsets
                .as_ref()
                .and_then(|v| v.get(index))
                .copied();

            Some(TrunEntry {
                duration,
                size,
                flags,
                composition_time_offset,
            })
        }

        /// Returns an iterator over the `TrunEntry` items for all samples in this `trun` box.
        pub fn entries(&self) -> impl Iterator<Item = TrunEntry> + '_ {
            let count = self.sample_count();
            (0..count).filter_map(|i| self.get_entry(i))
        }

        /// Adds a sample entry to this `trun` box. The fields of the entry must be `Some` if the corresponding flags are set in this box.
        pub fn push_entry(&mut self, entry: TrunEntry) {
            self.sample_count += 1;
            if let Some(ref mut durations) = self.sample_durations {
                durations.push(entry.duration.expect("missing duration for sample"));
            }
            if let Some(ref mut sizes) = self.sample_sizes {
                sizes.push(entry.size.expect("missing size for sample"));
            }
            if let Some(ref mut flags) = self.sample_flags {
                flags.push(entry.flags.expect("missing flags for sample"));
            }
            if let Some(ref mut offsets) = self.sample_composition_time_offsets {
                offsets.push(
                    entry
                        .composition_time_offset
                        .expect("missing composition time offset for sample"),
                );
            }
        }
    }

    impl TryFrom<&TrunBoxView<'_>> for TrunBox {
        type Error = Error;

        fn try_from(view: &TrunBoxView<'_>) -> Result<Self> {
            let mut sample_durations: Option<Vec<u32>> =
                if view.flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT) {
                    Some(Vec::with_capacity(view.sample_count as usize))
                } else {
                    None
                };
            let mut sample_sizes: Option<Vec<u32>> =
                if view.flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT) {
                    Some(Vec::with_capacity(view.sample_count as usize))
                } else {
                    None
                };
            let mut sample_flags: Option<Vec<SampleFlags>> =
                if view.flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT) {
                    Some(Vec::with_capacity(view.sample_count as usize))
                } else {
                    None
                };
            let mut sample_composition_time_offsets: Option<Vec<i64>> = if view
                .flags
                .contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT)
            {
                Some(Vec::with_capacity(view.sample_count as usize))
            } else {
                None
            };

            for entry in view.entries() {
                let entry = entry?;

                if let Some(ref mut durations) = sample_durations {
                    let sample_duration = entry.duration.ok_or_else(|| {
                        Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "sample.duration",
                                reason: "missing duration for sample",
                            },
                            BoxType::TRUN,
                        )
                    })?;

                    durations.push(sample_duration);
                }
                if let Some(ref mut sizes) = sample_sizes {
                    let sample_size = entry.size.ok_or_else(|| {
                        Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "sample.size",
                                reason: "missing size for sample",
                            },
                            BoxType::TRUN,
                        )
                    })?;

                    sizes.push(sample_size);
                }
                if let Some(ref mut flags) = sample_flags {
                    let sample_flags = entry.flags.ok_or_else(|| {
                        Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "sample.flags",
                                reason: "missing flags for sample",
                            },
                            BoxType::TRUN,
                        )
                    })?;

                    flags.push(sample_flags);
                }
                if let Some(ref mut offsets) = sample_composition_time_offsets {
                    let sample_composition_time_offset =
                        entry.composition_time_offset.ok_or_else(|| {
                            Error::in_box(
                                ErrorKind::InvalidBoxField {
                                    field: "sample.composition_time_offset",
                                    reason: "missing composition_time_offset for sample",
                                },
                                BoxType::TRUN,
                            )
                        })?;

                    offsets.push(sample_composition_time_offset);
                }
            }

            Ok(TrunBox {
                version: view.version,
                flags: view.flags,
                sample_count: view.sample_count,
                data_offset: view.data_offset,
                first_sample_flags: view.first_sample_flags,
                sample_durations,
                sample_sizes,
                sample_flags,
                sample_composition_time_offsets,
            })
        }
    }

    impl BoxCodec for TrunBox {
        fn boxtype(&self) -> BoxType {
            BoxType::TRUN
        }
    }

    impl BoxDecode<'_> for TrunBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = TrunBoxView::decode(bytes)?;
            TrunBox::try_from(&view)
        }
    }

    impl BoxEncode for TrunBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut size = 1 // version
                + 3 // flags
                + 4; // sample_count

            if self.flags.contains(TrunFlags::DATA_OFFSET_PRESENT) {
                size += 4;
            }
            if self.flags.contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT) {
                size += 4;
            }

            let sample_size = sample_data_size(self.flags);
            size += self.sample_count() * sample_size;

            size
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(u32::try_from(self.sample_count())?)?;

            if self.flags.contains(TrunFlags::DATA_OFFSET_PRESENT) {
                if let Some(data_offset) = self.data_offset {
                    cur.write_i32_be(data_offset)?;
                } else {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "data_offset",
                            reason: "missing data_offset for samples",
                        },
                        BoxType::TRUN,
                    ));
                }
            }

            if self.flags.contains(TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT) {
                if let Some(first_sample_flags) = self.first_sample_flags {
                    cur.write_u32_be(first_sample_flags.to_raw())?;
                } else {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "first_sample_flags",
                            reason: "missing first_sample_flags for samples",
                        },
                        BoxType::TRUN,
                    ));
                }
            }

            for sample in self.entries() {
                if self.flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT) {
                    if let Some(duration) = sample.duration {
                        cur.write_u32_be(duration)?;
                    } else {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "sample.duration",
                                reason: "missing duration for samples",
                            },
                            BoxType::TRUN,
                        ));
                    }
                }

                if self.flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT) {
                    if let Some(size) = sample.size {
                        cur.write_u32_be(size)?;
                    } else {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "sample.size",
                                reason: "missing size for samples",
                            },
                            BoxType::TRUN,
                        ));
                    }
                }

                if self.flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT) {
                    if let Some(flags) = sample.flags {
                        cur.write_u32_be(flags.to_raw())?;
                    } else {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "sample.flags",
                                reason: "missing flags for samples",
                            },
                            BoxType::TRUN,
                        ));
                    }
                }

                if self
                    .flags
                    .contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT)
                {
                    if let Some(offset) = sample.composition_time_offset {
                        if self.version == 0 {
                            let v = u32::try_from(offset)
                                .map_err(|e| Error::from(e).with_box_type(BoxType::TRUN))?;
                            cur.write_u32_be(v)?;
                        } else {
                            let v = i32::try_from(offset)
                                .map_err(|e| Error::from(e).with_box_type(BoxType::TRUN))?;
                            cur.write_i32_be(v)?;
                        }
                    } else {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "sample.composition_time_offset",
                                reason: "missing composition_time_offset for samples",
                            },
                            BoxType::TRUN,
                        ));
                    }
                }
            }

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_minimal() -> [u8; 8] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ]
    }

    fn raw_data_with_samples() -> [u8; 28] {
        [
            0x00, // version = 0
            0x00, 0x03, 0x01, // flags = DATA_OFFSET | SAMPLE_DURATION | SAMPLE_SIZE
            0x00, 0x00, 0x00, 0x02, // sample_count = 2
            0x00, 0x00, 0x01, 0x00, // data_offset = 256
            // sample 1
            0x00, 0x00, 0x03, 0xE8, // duration = 1000
            0x00, 0x00, 0x10, 0x00, // size = 4096
            // sample 2
            0x00, 0x00, 0x03, 0xE8, // duration = 1000
            0x00, 0x00, 0x08, 0x00, // size = 2048
        ]
    }

    #[test]
    fn test_trun_box_view_decode_minimal() {
        let data = raw_data_minimal();
        let trun = TrunBoxView::decode(&data).unwrap();

        assert_eq!(trun.version, 0);
        assert_eq!(trun.flags.bits(), 0);
        assert_eq!(trun.sample_count, 0);
        assert!(trun.data_offset.is_none());
        assert!(trun.first_sample_flags.is_none());
        assert_eq!(trun.entries().count(), 0);
    }

    #[test]
    fn test_trun_box_view_decode_with_samples() {
        let data = raw_data_with_samples();
        let trun = TrunBoxView::decode(&data).unwrap();

        assert_eq!(trun.version, 0);
        assert_eq!(trun.sample_count, 2);
        assert_eq!(trun.data_offset, Some(256));

        let samples: Vec<_> = trun.entries().collect();
        assert_eq!(samples.len(), 2);

        let sample1 = samples[0].as_ref().unwrap();
        assert_eq!(sample1.duration, Some(1000));
        assert_eq!(sample1.size, Some(4096));

        let sample2 = samples[1].as_ref().unwrap();
        assert_eq!(sample2.duration, Some(1000));
        assert_eq!(sample2.size, Some(2048));
    }

    #[test]
    fn test_trun_box_view_decode_invalid_version() {
        let data: [u8; 8] = [
            0x02, // version = 2 (invalid)
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ];

        let result = TrunBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_trun_box_view_decode_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = TrunBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_trun_box_view_decode_sample_count_mismatch() {
        let data: [u8; 12] = [
            0x00, // version = 0
            0x00, 0x01, 0x00, // flags = SAMPLE_DURATION
            0x00, 0x00, 0x00, 0x02, // sample_count = 2
            // only 1 sample provided (4 bytes instead of 8)
            0x00, 0x00, 0x03, 0xE8,
        ];

        let result = TrunBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_trun_box_round_trip_minimal() {
        use crate::BoxEncode;

        let original = raw_data_minimal();
        let trun = TrunBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; trun.encoded_len()];
        let len = trun.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_trun_box_round_trip_with_samples() {
        use crate::BoxEncode;

        let original = raw_data_with_samples();
        let trun = TrunBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; trun.encoded_len()];
        let len = trun.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_trun_box_try_from() {
        let data = raw_data_with_samples();
        let view = TrunBoxView::decode(&data).unwrap();
        let owned = TrunBox::try_from(&view).unwrap();

        assert_eq!(owned.version(), view.version);
        assert_eq!(owned.flags().bits(), view.flags.bits());
        assert_eq!(owned.sample_count(), view.sample_count as usize);
    }
}
