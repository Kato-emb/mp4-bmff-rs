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
pub struct TrunSample {
    /// Sample duration in track timescale units.
    pub duration: Option<u32>,
    /// Sample size in bytes.
    pub size: Option<u32>,
    /// Sample flags (sync, dependency info, degradation priority).
    pub flags: Option<u32>,
    /// Composition time offset relative to decode time (signed in v1).
    pub composition_time_offset: Option<i32>,
}

/// An iterator over samples in a Track Run Box (`trun`).
#[derive(Debug)]
pub struct TrunSampleIter<'a> {
    samples: &'a [u8],
    flags: TrunFlags,
    version: u8,
}

impl Iterator for TrunSampleIter<'_> {
    type Item = Result<TrunSample>;

    fn next(&mut self) -> Option<Self::Item> {
        // When samples slice is empty, return None immediately
        if self.samples.is_empty() {
            return None;
        }

        let sample_size = sample_data_size(self.flags);
        let chunk = &self.samples[..sample_size];
        self.samples = &self.samples[sample_size..];

        let mut cursor = ReadCursor::new(chunk);

        let sample_duration = if self.flags.contains(TrunFlags::SAMPLE_DURATION_PRESENT) {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let sample_size = if self.flags.contains(TrunFlags::SAMPLE_SIZE_PRESENT) {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let sample_flags = if self.flags.contains(TrunFlags::SAMPLE_FLAGS_PRESENT) {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let sample_composition_time_offset = if self
            .flags
            .contains(TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT)
        {
            if self.version == 0 {
                match cursor.read_u32_be() {
                    Ok(v) => Some(v as i32),
                    Err(e) => return Some(Err(e.into())),
                }
            } else {
                match cursor.read_i32_be() {
                    Ok(v) => Some(v),
                    Err(e) => return Some(Err(e.into())),
                }
            }
        } else {
            None
        };

        Some(Ok(TrunSample {
            duration: sample_duration,
            size: sample_size,
            flags: sample_flags,
            composition_time_offset: sample_composition_time_offset,
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

impl ExactSizeIterator for TrunSampleIter<'_> {}

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
    pub first_sample_flags: Option<u32>,
    samples: &'a [u8],
}

impl<'a> TrunBoxView<'a> {
    /// Returns an iterator over the samples in the Track Run Box (`trun`).
    pub fn samples(&self) -> TrunSampleIter<'a> {
        TrunSampleIter {
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
            Some(cur.read_u32_be()?)
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
    use crate::lib::Vec;

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
        pub version: u8,
        /// Flags indicating which fields are present.
        pub flags: TrunFlags,
        /// Signed offset from base to first sample's data.
        pub data_offset: Option<i32>,
        /// Flags for first sample (overrides sample\[0\].flags if present).
        pub first_sample_flags: Option<u32>,
        /// Per-sample information for this run.
        pub samples: Vec<TrunSample>,
    }

    impl TryFrom<&TrunBoxView<'_>> for TrunBox {
        type Error = Error;

        fn try_from(view: &TrunBoxView<'_>) -> Result<Self> {
            let samples: Result<Vec<TrunSample>> = view.samples().collect();
            Ok(TrunBox {
                version: view.version,
                flags: view.flags,
                data_offset: view.data_offset,
                first_sample_flags: view.first_sample_flags,
                samples: samples?,
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
            size += self.samples.len() * sample_size;

            size
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.samples.len() as u32)?;

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
                    cur.write_u32_be(first_sample_flags)?;
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

            for sample in &self.samples {
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
                        cur.write_u32_be(flags)?;
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
                            cur.write_u32_be(offset as u32)?;
                        } else {
                            cur.write_i32_be(offset)?;
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
        assert_eq!(trun.samples().count(), 0);
    }

    #[test]
    fn test_trun_box_view_decode_with_samples() {
        let data = raw_data_with_samples();
        let trun = TrunBoxView::decode(&data).unwrap();

        assert_eq!(trun.version, 0);
        assert_eq!(trun.sample_count, 2);
        assert_eq!(trun.data_offset, Some(256));

        let samples: Vec<_> = trun.samples().collect();
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

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.samples.len(), view.sample_count as usize);
    }
}
