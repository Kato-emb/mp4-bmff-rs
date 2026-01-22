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
        /// The raw sample data bytes.
        samples: Vec<u8>,
    }

    impl From<&TrunBoxView<'_>> for TrunBox {
        fn from(view: &TrunBoxView<'_>) -> Self {
            let samples = view.samples.to_vec();

            TrunBox {
                version: view.version,
                flags: view.flags,
                sample_count: view.sample_count,
                data_offset: view.data_offset,
                first_sample_flags: view.first_sample_flags,
                samples,
            }
        }
    }

    impl TrunBox {
        /// Returns an iterator over the samples in the Track Run Box.
        ///
        /// Each sample yields a tuple of (duration, size, flags, composition_time_offset).
        pub fn samples(&self) -> TrunSampleIter<'_> {
            TrunSampleIter {
                samples: &self.samples,
                flags: self.flags,
                version: self.version,
            }
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
            Ok(TrunBox::from(&view))
        }
    }

    impl BoxEncode for TrunBox {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
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

            cur.write_slice(&self.samples)?;

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
        let written = original.encode(&mut buf).unwrap();

        // Parse again
        let reparsed = TrunBox::decode(&buf[..written]).unwrap();

        // Compare
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.sample_count, original.sample_count);
        assert_eq!(reparsed.data_offset, original.data_offset);
        assert_eq!(reparsed.first_sample_flags, original.first_sample_flags);

        let orig_samples: Vec<_> = original.samples().collect();
        let reparsed_samples: Vec<_> = reparsed.samples().collect();
        assert_eq!(orig_samples.len(), reparsed_samples.len());

        for (o, r) in orig_samples.iter().zip(reparsed_samples.iter()) {
            let o = o.as_ref().unwrap();
            let r = r.as_ref().unwrap();
            assert_eq!(o.duration, r.duration);
            assert_eq!(o.size, r.size);
            assert_eq!(o.flags, r.flags);
            assert_eq!(o.composition_time_offset, r.composition_time_offset);
        }
    }
}
