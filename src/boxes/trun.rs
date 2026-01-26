use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// Sample data from a Track Run Box (`trun`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrunSample {
    /// The duration of the sample.
    pub duration: Option<u32>,
    /// The size of the sample in bytes.
    pub size: Option<u32>,
    /// The sample flags.
    pub flags: Option<u32>,
    /// The composition time offset (signed in version 1, unsigned in version 0).
    pub composition_time_offset: Option<i32>,
}

/// An iterator over samples in a Track Run Box (`trun`).
#[derive(Debug)]
pub struct TrunSampleIter<'a> {
    samples: &'a [u8],
    flags: TrunFlags,
    version: u8,
}

impl<'a> Iterator for TrunSampleIter<'a> {
    type Item = Result<TrunSample>;

    fn next(&mut self) -> Option<Self::Item> {
        // When samples slice is empty, return None immediately
        if self.samples.is_empty() {
            return None;
        }

        let sample_size = self.flags.sample_data_size();
        let chunk = &self.samples[..sample_size];
        self.samples = &self.samples[sample_size..];

        let mut cursor = ReadCursor::new(chunk);

        let sample_duration = if self.flags.sample_duration_present() {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let sample_size = if self.flags.sample_size_present() {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let sample_flags = if self.flags.sample_flags_present() {
            match cursor.read_u32_be() {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e.into())),
            }
        } else {
            None
        };

        let sample_composition_time_offset = if self.flags.sample_composition_time_offsets_present()
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
        let sample_size = self.flags.sample_data_size();
        if sample_size == 0 {
            (0, Some(0))
        } else {
            let count = self.samples.len() / sample_size;
            (count, Some(count))
        }
    }
}

impl<'a> ExactSizeIterator for TrunSampleIter<'a> {}

/// A reference to a Track Run Box (`trun`).
#[derive(Debug)]
pub struct TrunBoxView<'a> {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: TrunFlags,
    /// The number of samples in this run.
    pub sample_count: u32,
    /// The data offset from the start of the moof.
    pub data_offset: Option<i32>,
    /// The flags for the first sample (overrides default_sample_flags).
    pub first_sample_flags: Option<u32>,
    samples: &'a [u8],
}

impl<'a> TrunBoxView<'a> {
    /// Returns an iterator over the samples in the Track Run Box.
    ///
    /// Each sample yields a tuple of (duration, size, flags, composition_time_offset).
    /// When sample_count is 0, the iterator returns None immediately.
    pub fn samples(&self) -> TrunSampleIter<'a> {
        TrunSampleIter {
            samples: self.samples,
            flags: self.flags,
            version: self.version,
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for TrunBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        TrunBoxView::decode(value)
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
        let flags = TrunFlags::from_bytes(cur.read_array()?);

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

        let data_offset = if flags.data_offset_present() {
            Some(cur.read_i32_be()?)
        } else {
            None
        };

        let first_sample_flags = if flags.first_sample_flags_present() {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let expected_size = sample_count as usize * flags.sample_data_size();
        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "trun samples size does not match sample_count",
                    got: cur.remaining() as u64,
                },
                BoxType::TRUN,
            ));
        }

        let samples = cur.take(cur.remaining())?;

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

/// Specification for the Track Run Box (`trun`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrunSpec;

/// Flags for the Track Run Box (`trun`).
pub type TrunFlags = FullBoxFlags<TrunSpec>;

impl TrunFlags {
    /// If set, data_offset is present.
    pub const DATA_OFFSET_PRESENT: TrunFlags = TrunFlags::from_bits_truncate(0x000001);
    /// If set, first_sample_flags is present.
    pub const FIRST_SAMPLE_FLAGS_PRESENT: TrunFlags = TrunFlags::from_bits_truncate(0x000004);
    /// If set, each sample has a duration field.
    pub const SAMPLE_DURATION_PRESENT: TrunFlags = TrunFlags::from_bits_truncate(0x000100);
    /// If set, each sample has a size field.
    pub const SAMPLE_SIZE_PRESENT: TrunFlags = TrunFlags::from_bits_truncate(0x000200);
    /// If set, each sample has a flags field.
    pub const SAMPLE_FLAGS_PRESENT: TrunFlags = TrunFlags::from_bits_truncate(0x000400);
    /// If set, each sample has a composition time offset field.
    pub const SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT: TrunFlags =
        TrunFlags::from_bits_truncate(0x000800);

    /// Returns true if data_offset is present.
    pub fn data_offset_present(&self) -> bool {
        self.contains(Self::DATA_OFFSET_PRESENT)
    }

    /// Returns true if first_sample_flags is present.
    pub fn first_sample_flags_present(&self) -> bool {
        self.contains(Self::FIRST_SAMPLE_FLAGS_PRESENT)
    }

    /// Returns true if sample_duration is present for each sample.
    pub fn sample_duration_present(&self) -> bool {
        self.contains(Self::SAMPLE_DURATION_PRESENT)
    }

    /// Returns true if sample_size is present for each sample.
    pub fn sample_size_present(&self) -> bool {
        self.contains(Self::SAMPLE_SIZE_PRESENT)
    }

    /// Returns true if sample_flags is present for each sample.
    pub fn sample_flags_present(&self) -> bool {
        self.contains(Self::SAMPLE_FLAGS_PRESENT)
    }

    /// Returns true if sample_composition_time_offset is present for each sample.
    pub fn sample_composition_time_offsets_present(&self) -> bool {
        self.contains(Self::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT)
    }

    /// Returns the size in bytes of each sample entry based on the flags.
    pub fn sample_data_size(&self) -> usize {
        let mut size = 0;
        if self.sample_duration_present() {
            size += 4;
        }
        if self.sample_size_present() {
            size += 4;
        }
        if self.sample_flags_present() {
            size += 4;
        }
        if self.sample_composition_time_offsets_present() {
            size += 4;
        }
        size
    }
}

#[cfg(feature = "alloc")]
pub use owned::TrunBox;

#[cfg(feature = "alloc")]
mod owned {
    extern crate alloc;
    use alloc::vec::Vec;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use super::*;

    /// An owned Track Run Box (`trun`).
    #[derive(Debug, Clone)]
    pub struct TrunBox {
        /// The version of the box (0 or 1).
        pub version: u8,
        /// The flags of the box.
        pub flags: TrunFlags,
        /// The number of samples in this run.
        pub sample_count: u32,
        /// The data offset from the start of the moof.
        pub data_offset: Option<i32>,
        /// The flags for the first sample.
        pub first_sample_flags: Option<u32>,
        /// The samples in this run.
        /// When `flags.sample_data_size() == 0`, this may be empty even if `sample_count > 0`,
        /// indicating that all samples use default values from tfhd/trex.
        pub samples: Vec<TrunSample>,
    }

    impl TryFrom<&TrunBoxView<'_>> for TrunBox {
        type Error = Error;

        fn try_from(view: &TrunBoxView<'_>) -> Result<Self> {
            let samples: Vec<TrunSample> = if view.flags.sample_data_size() == 0 {
                // No per-sample data, samples use default values
                Vec::new()
            } else {
                view.samples().collect::<Result<Vec<_>>>()?
            };

            Ok(TrunBox {
                version: view.version,
                flags: view.flags,
                sample_count: view.sample_count,
                data_offset: view.data_offset,
                first_sample_flags: view.first_sample_flags,
                samples,
            })
        }
    }

    impl TrunBox {
        /// Creates a new Track Run Box with sample data.
        ///
        /// Use this when samples have per-sample fields (duration, size, flags, etc.).
        /// The `sample_count` is automatically set to `samples.len()`.
        ///
        /// For samples using only default values (no per-sample data), use [`TrunBox::with_defaults`] instead.
        ///
        /// # Arguments
        ///
        /// * `version` - The version of the box (0 or 1).
        /// * `flags` - The flags indicating which optional fields are present.
        /// * `data_offset` - The data offset from the start of the moof (required if `DATA_OFFSET_PRESENT` flag is set).
        /// * `first_sample_flags` - The flags for the first sample (required if `FIRST_SAMPLE_FLAGS_PRESENT` flag is set).
        /// * `samples` - The sample data.
        ///
        /// # Errors
        ///
        /// Returns an error if:
        /// - `version` is not 0 or 1.
        /// - `data_offset` presence doesn't match `DATA_OFFSET_PRESENT` flag.
        /// - `first_sample_flags` presence doesn't match `FIRST_SAMPLE_FLAGS_PRESENT` flag.
        /// - Any sample is missing a required field based on the flags.
        pub fn new(
            version: u8,
            flags: TrunFlags,
            data_offset: Option<i32>,
            first_sample_flags: Option<u32>,
            samples: Vec<TrunSample>,
        ) -> Result<Self> {
            // Validate version
            if version > 1 {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "trun version must be 0 or 1",
                        got: version,
                    },
                    BoxType::TRUN,
                ));
            }

            // Validate data_offset presence matches flag
            if data_offset.is_some() != flags.data_offset_present() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "data_offset",
                        reason: "presence must match DATA_OFFSET_PRESENT flag",
                    },
                    BoxType::TRUN,
                ));
            }

            // Validate first_sample_flags presence matches flag
            if first_sample_flags.is_some() != flags.first_sample_flags_present() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "first_sample_flags",
                        reason: "presence must match FIRST_SAMPLE_FLAGS_PRESENT flag",
                    },
                    BoxType::TRUN,
                ));
            }

            // Validate samples
            for sample in &samples {
                if flags.sample_duration_present() && sample.duration.is_none() {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "sample.duration",
                            reason: "required by SAMPLE_DURATION_PRESENT flag but missing",
                        },
                        BoxType::TRUN,
                    ));
                }

                if flags.sample_size_present() && sample.size.is_none() {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "sample.size",
                            reason: "required by SAMPLE_SIZE_PRESENT flag but missing",
                        },
                        BoxType::TRUN,
                    ));
                }

                if flags.sample_flags_present() && sample.flags.is_none() {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "sample.flags",
                            reason: "required by SAMPLE_FLAGS_PRESENT flag but missing",
                        },
                        BoxType::TRUN,
                    ));
                }

                if flags.sample_composition_time_offsets_present()
                    && sample.composition_time_offset.is_none()
                {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "sample.composition_time_offset",
                            reason: "required by SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT flag but missing",
                        },
                        BoxType::TRUN,
                    ));
                }
            }

            let sample_count = samples.len() as u32;

            Ok(Self {
                version,
                flags,
                sample_count,
                data_offset,
                first_sample_flags,
                samples,
            })
        }

        /// Creates a new Track Run Box with default sample values.
        ///
        /// This is useful when all samples use default values from tfhd/trex.
        pub fn with_defaults(
            version: u8,
            sample_count: u32,
            data_offset: Option<i32>,
            first_sample_flags: Option<u32>,
        ) -> Result<Self> {
            let mut flags = TrunFlags::default();
            if data_offset.is_some() {
                flags |= TrunFlags::DATA_OFFSET_PRESENT;
            }
            if first_sample_flags.is_some() {
                flags |= TrunFlags::FIRST_SAMPLE_FLAGS_PRESENT;
            }

            Ok(Self {
                version,
                flags,
                sample_count,
                data_offset,
                first_sample_flags,
                samples: Vec::new(),
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
            1 // version
                + 3 // flags
                + 4 // sample_count
                + if self.flags.data_offset_present() {
                    4
                } else {
                    0
                }
                + if self.flags.first_sample_flags_present() {
                    4
                } else {
                    0
                }
                + self.samples.len() * self.flags.sample_data_size()
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;

            cur.write_u32_be(self.sample_count)?;

            if let Some(data_offset) = self.data_offset {
                cur.write_i32_be(data_offset)?;
            }

            if let Some(first_sample_flags) = self.first_sample_flags {
                cur.write_u32_be(first_sample_flags)?;
            }

            for sample in &self.samples {
                if self.flags.sample_duration_present() {
                    cur.write_u32_be(sample.duration.unwrap_or(0))?;
                }
                if self.flags.sample_size_present() {
                    cur.write_u32_be(sample.size.unwrap_or(0))?;
                }
                if self.flags.sample_flags_present() {
                    cur.write_u32_be(sample.flags.unwrap_or(0))?;
                }
                if self.flags.sample_composition_time_offsets_present() {
                    cur.write_i32_be(sample.composition_time_offset.unwrap_or(0))?;
                }
            }

            Ok(cur.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trun_payload(
        version: u8,
        flags: u32,
        sample_count: u32,
        optional_header: &[u8],
        samples: &[u8],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data.extend_from_slice(&sample_count.to_be_bytes());
        data.extend_from_slice(optional_header);
        data.extend_from_slice(samples);
        data
    }

    #[test]
    fn parse_trun_with_all_sample_fields() {
        let flags = 0x000100 | 0x000200 | 0x000400 | 0x000800; // all sample fields

        let mut samples = Vec::new();
        // Sample 1
        samples.extend_from_slice(&100u32.to_be_bytes()); // duration
        samples.extend_from_slice(&1000u32.to_be_bytes()); // size
        samples.extend_from_slice(&0x02000000u32.to_be_bytes()); // flags
        samples.extend_from_slice(&50u32.to_be_bytes()); // composition offset

        let payload = make_trun_payload(0, flags, 1, &[], &samples);
        let trun = TrunBoxView::decode(&payload).unwrap();

        let sample = trun.samples().next().unwrap().unwrap();
        assert_eq!(sample.duration, Some(100));
        assert_eq!(sample.size, Some(1000));
        assert_eq!(sample.flags, Some(0x02000000));
        assert_eq!(sample.composition_time_offset, Some(50));
    }

    #[test]
    fn parse_trun_v1_negative_composition_offset() {
        let flags = 0x000800; // composition time offset present

        let mut samples = Vec::new();
        samples.extend_from_slice(&(-100i32).to_be_bytes()); // negative offset

        let payload = make_trun_payload(1, flags, 1, &[], &samples);
        let trun = TrunBoxView::decode(&payload).unwrap();

        let sample = trun.samples().next().unwrap().unwrap();
        assert_eq!(sample.composition_time_offset, Some(-100));
    }

    #[test]
    fn parse_trun_invalid_version() {
        let payload = make_trun_payload(2, 0, 0, &[], &[]);
        let result = TrunBoxView::decode(&payload);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidBoxVersion { .. }
        ));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn trun_box_round_trip() {
        // Create payload with data_offset, first_sample_flags, and multiple samples with all fields

        use crate::BoxEncode;
        let flags = 0x000001 | 0x000004 | 0x000100 | 0x000200 | 0x000400 | 0x000800;

        let mut header = Vec::new();
        header.extend_from_slice(&100i32.to_be_bytes()); // data_offset
        header.extend_from_slice(&0x01000000u32.to_be_bytes()); // first_sample_flags

        let mut samples = Vec::new();
        // Sample 1
        samples.extend_from_slice(&100u32.to_be_bytes()); // duration
        samples.extend_from_slice(&1000u32.to_be_bytes()); // size
        samples.extend_from_slice(&0x02000000u32.to_be_bytes()); // flags
        samples.extend_from_slice(&50i32.to_be_bytes()); // composition offset
        // Sample 2
        samples.extend_from_slice(&200u32.to_be_bytes()); // duration
        samples.extend_from_slice(&2000u32.to_be_bytes()); // size
        samples.extend_from_slice(&0x03000000u32.to_be_bytes()); // flags
        samples.extend_from_slice(&(-25i32).to_be_bytes()); // negative composition offset

        let original_payload = make_trun_payload(1, flags, 2, &header, &samples);

        // Parse
        let original = TrunBox::decode(&original_payload).unwrap();

        // Write
        let mut buf = vec![0u8; 256];
        let written = original.encode_into(&mut buf).unwrap();

        // Parse again
        let reparsed = TrunBox::decode(&buf[..written]).unwrap();

        // Compare
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.sample_count, original.sample_count);
        assert_eq!(reparsed.data_offset, original.data_offset);
        assert_eq!(reparsed.first_sample_flags, original.first_sample_flags);

        assert_eq!(original.samples.len(), reparsed.samples.len());

        for (o, r) in original.samples.iter().zip(reparsed.samples.iter()) {
            assert_eq!(o.duration, r.duration);
            assert_eq!(o.size, r.size);
            assert_eq!(o.flags, r.flags);
            assert_eq!(o.composition_time_offset, r.composition_time_offset);
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn trun_box_new_valid() {
        use crate::BoxEncode;

        let flags = TrunFlags::DATA_OFFSET_PRESENT
            | TrunFlags::SAMPLE_DURATION_PRESENT
            | TrunFlags::SAMPLE_SIZE_PRESENT
            | TrunFlags::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT;

        let samples = vec![
            TrunSample {
                duration: Some(1000),
                size: Some(5000),
                flags: None,
                composition_time_offset: Some(100),
            },
            TrunSample {
                duration: Some(1000),
                size: Some(3000),
                flags: None,
                composition_time_offset: Some(-50),
            },
        ];

        let trun = TrunBox::new(1, flags, Some(1024), None, samples).unwrap();

        assert_eq!(trun.version, 1);
        assert_eq!(trun.sample_count, 2);
        assert_eq!(trun.data_offset, Some(1024));
        assert_eq!(trun.first_sample_flags, None);

        // Round-trip test
        let mut buf = vec![0u8; trun.encoded_len()];
        trun.encode(&mut buf).unwrap();

        let reparsed = TrunBox::decode(&buf).unwrap();

        assert_eq!(reparsed.sample_count, 2);
        assert_eq!(reparsed.samples.len(), 2);
        assert_eq!(reparsed.samples[0].duration, Some(1000));
        assert_eq!(reparsed.samples[0].size, Some(5000));
        assert_eq!(reparsed.samples[0].composition_time_offset, Some(100));
        assert_eq!(reparsed.samples[1].composition_time_offset, Some(-50));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn trun_box_new_with_defaults() {
        use crate::BoxEncode;

        // Create a trun with 10 samples, all using default values
        let trun = TrunBox::with_defaults(0, 10, Some(100), None).unwrap();

        assert_eq!(trun.version, 0);
        assert_eq!(trun.sample_count, 10);
        assert_eq!(trun.data_offset, Some(100));
        assert!(trun.samples.is_empty()); // No per-sample data

        // Round-trip test
        let mut buf = vec![0u8; trun.encoded_len()];
        trun.encode(&mut buf).unwrap();

        let reparsed = TrunBox::decode(&buf).unwrap();

        assert_eq!(reparsed.sample_count, 10);
        assert!(reparsed.samples.is_empty());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn trun_box_new_invalid_version() {
        let result = TrunBox::new(2, TrunFlags::default(), None, None, vec![]);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidBoxVersion { .. }
        ));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn trun_box_new_data_offset_mismatch() {
        // Flag says data_offset present, but None provided
        let flags = TrunFlags::DATA_OFFSET_PRESENT;
        let result = TrunBox::new(0, flags, None, None, vec![]);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidBoxField {
                field: "data_offset",
                ..
            }
        ));

        // Flag says data_offset not present, but Some provided
        let flags = TrunFlags::default();
        let result = TrunBox::new(0, flags, Some(100), None, vec![]);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidBoxField {
                field: "data_offset",
                ..
            }
        ));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn trun_box_new_missing_sample_field() {
        let flags = TrunFlags::SAMPLE_DURATION_PRESENT | TrunFlags::SAMPLE_SIZE_PRESENT;

        // Sample missing required duration
        let samples = vec![TrunSample {
            duration: None, // Missing!
            size: Some(1000),
            flags: None,
            composition_time_offset: None,
        }];

        let result = TrunBox::new(0, flags, None, None, samples);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidBoxField {
                field: "sample.duration",
                ..
            }
        ));
    }
}
