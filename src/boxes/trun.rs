use crate::cursor::ReadCursor;

use crate::BoxFrame;
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
            match cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))
            {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e)),
            }
        } else {
            None
        };

        let sample_size = if self.flags.sample_size_present() {
            match cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))
            {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e)),
            }
        } else {
            None
        };

        let sample_flags = if self.flags.sample_flags_present() {
            match cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))
            {
                Ok(v) => Some(v),
                Err(e) => return Some(Err(e)),
            }
        } else {
            None
        };

        let sample_composition_time_offset = if self.flags.sample_composition_time_offsets_present()
        {
            if self.version == 0 {
                match cursor
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cursor.position() as u64))
                {
                    Ok(v) => Some(v as i32),
                    Err(e) => return Some(Err(e)),
                }
            } else {
                match cursor
                    .read_i32_be()
                    .map_err(|e| Error::at(e.into(), cursor.position() as u64))
                {
                    Ok(v) => Some(v),
                    Err(e) => return Some(Err(e)),
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<TrunBoxView<'a>> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = TrunFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "trun version must be 0 or 1",
                    got: version,
                },
                BoxType::TRUN,
            ));
        }

        let sample_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let data_offset = if flags.data_offset_present() {
            Some(
                cur.read_i32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        let first_sample_flags = if flags.first_sample_flags_present() {
            Some(
                cur.read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
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

    /// Parses a `TrunBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<TrunBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        TrunBoxView::parse_in(&mut cur)
    }
}

impl<'a> TryFrom<&'a [u8]> for TrunBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        TrunBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for TrunBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::TRUN {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TRUN,
                found: value.boxtype(),
            }));
        }

        TrunBoxView::parse(value.payload())
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

    impl TrunBox {
        /// Creates a `TrunBox` from a `TrunBoxView`.
        pub fn from_view(view: &TrunBoxView<'_>) -> Result<TrunBox> {
            Ok(TrunBox {
                version: view.version,
                flags: view.flags,
                sample_count: view.sample_count,
                data_offset: view.data_offset,
                first_sample_flags: view.first_sample_flags,
                samples: view.samples.to_vec(),
            })
        }

        /// Parses a `TrunBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<TrunBox> {
            let view = TrunBoxView::parse(payload)?;
            TrunBox::from_view(&view)
        }

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

    impl TryFrom<&TrunBoxView<'_>> for TrunBox {
        type Error = Error;

        fn try_from(value: &TrunBoxView<'_>) -> Result<Self> {
            TrunBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for TrunBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            let view = TrunBoxView::try_from(value)?;
            TrunBox::from_view(&view)
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
    fn parse_trun_minimal() {
        let payload = make_trun_payload(0, 0, 0, &[], &[]);
        let trun = TrunBoxView::parse(&payload).unwrap();

        assert_eq!(trun.version, 0);
        assert_eq!(trun.sample_count, 0);
        assert!(trun.data_offset.is_none());
        assert!(trun.first_sample_flags.is_none());
        assert_eq!(trun.samples().count(), 0);
    }

    #[test]
    fn parse_trun_with_data_offset() {
        let mut header = Vec::new();
        header.extend_from_slice(&100i32.to_be_bytes());

        let payload = make_trun_payload(0, 0x000001, 0, &header, &[]);
        let trun = TrunBoxView::parse(&payload).unwrap();

        assert_eq!(trun.data_offset, Some(100));
    }

    #[test]
    fn parse_trun_with_samples() {
        let flags = 0x000200; // sample_size_present

        let mut samples = Vec::new();
        samples.extend_from_slice(&1000u32.to_be_bytes()); // sample 1 size
        samples.extend_from_slice(&2000u32.to_be_bytes()); // sample 2 size

        let payload = make_trun_payload(0, flags, 2, &[], &samples);
        let trun = TrunBoxView::parse(&payload).unwrap();

        assert_eq!(trun.sample_count, 2);

        let parsed_samples: Vec<_> = trun.samples().collect();
        assert_eq!(parsed_samples.len(), 2);
        assert_eq!(parsed_samples[0].as_ref().unwrap().size, Some(1000));
        assert_eq!(parsed_samples[1].as_ref().unwrap().size, Some(2000));
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
        let trun = TrunBoxView::parse(&payload).unwrap();

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
        let trun = TrunBoxView::parse(&payload).unwrap();

        let sample = trun.samples().next().unwrap().unwrap();
        assert_eq!(sample.composition_time_offset, Some(-100));
    }

    #[test]
    fn parse_trun_invalid_version() {
        let payload = make_trun_payload(2, 0, 0, &[], &[]);
        let result = TrunBoxView::parse(&payload);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxVersion { .. }));
        }
    }

    #[test]
    fn parse_trun_size_mismatch() {
        let flags = 0x000200; // sample_size_present

        // Only 1 sample but sample_count is 2
        let mut samples = Vec::new();
        samples.extend_from_slice(&1000u32.to_be_bytes());

        let payload = make_trun_payload(0, flags, 2, &[], &samples);
        let result = TrunBoxView::parse(&payload);

        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_trun_payload(0, 0, 0, &[], &[]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"trun");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let trun = TrunBoxView::try_from(box_view).unwrap();

        assert_eq!(trun.sample_count, 0);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_trun_payload(0, 0, 0, &[], &[]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"tfhd");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = TrunBoxView::try_from(box_view);

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn trun_box_from_view() {
            let flags = 0x000200; // sample_size_present

            let mut samples = Vec::new();
            samples.extend_from_slice(&1000u32.to_be_bytes());
            samples.extend_from_slice(&2000u32.to_be_bytes());

            let payload = make_trun_payload(0, flags, 2, &[], &samples);
            let view = TrunBoxView::parse(&payload).unwrap();
            let owned = TrunBox::from_view(&view).unwrap();

            assert_eq!(owned.sample_count, 2);
            let parsed: Vec<_> = owned.samples().collect();
            assert_eq!(parsed[0].as_ref().unwrap().size, Some(1000));
            assert_eq!(parsed[1].as_ref().unwrap().size, Some(2000));
        }

        #[test]
        fn trun_box_parse() {
            let payload = make_trun_payload(0, 0, 0, &[], &[]);
            let owned = TrunBox::parse(&payload).unwrap();

            assert_eq!(owned.sample_count, 0);
            assert_eq!(owned.samples().count(), 0);
        }
    }
}
