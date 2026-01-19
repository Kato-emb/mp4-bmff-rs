use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// A reference to a Sample Size Box (`stsz`).
#[derive(Debug)]
pub struct StszBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: StszFlags,
    /// The default sample size. If 0, each sample has its own size in the entries.
    pub sample_size: u32,
    /// The number of samples in the track.
    pub sample_count: u32,
    /// The entry sizes (only present if sample_size is 0).
    entries: &'a [u8],
}

impl<'a> StszBoxView<'a> {
    const ENTRY_SIZE: usize = 4;

    /// Returns the size of a specific sample (1-indexed as per ISO spec).
    ///
    /// If `sample_size` is non-zero, returns that value for all samples.
    /// Otherwise, returns the size from the entries array.
    pub fn get_sample_size(&self, sample_number: u32) -> Result<u32> {
        if sample_number == 0 || sample_number > self.sample_count {
            return Err(Error::in_box(
                ErrorKind::Other {
                    description: "Sample number out of range",
                },
                BoxType::STSZ,
            ));
        }

        if self.sample_size != 0 {
            return Ok(self.sample_size);
        }

        let index = (sample_number - 1) as usize;
        let offset = index * Self::ENTRY_SIZE;

        if offset + Self::ENTRY_SIZE > self.entries.len() {
            return Err(Error::in_box(
                ErrorKind::Other {
                    description: "Entry index out of bounds",
                },
                BoxType::STSZ,
            ));
        }

        let mut cursor = ReadCursor::new(&self.entries[offset..offset + Self::ENTRY_SIZE]);
        let entry_size = cursor
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), offset as u64))?;

        Ok(entry_size)
    }

    /// Returns an iterator over all sample sizes.
    ///
    /// If `sample_size` is non-zero, yields that value for each sample.
    /// Otherwise, yields the size from the entries array.
    pub fn sample_sizes(&self) -> impl Iterator<Item = Result<u32>> + 'a {
        let sample_size = self.sample_size;
        let sample_count = self.sample_count as usize;
        let entries = self.entries;

        (0..sample_count).map(move |i| {
            if sample_size != 0 {
                Ok(sample_size)
            } else {
                let offset = i * Self::ENTRY_SIZE;
                Ok(u32::from_be_bytes([
                    entries[offset],
                    entries[offset + 1],
                    entries[offset + 2],
                    entries[offset + 3],
                ]))
            }
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<StszBoxView<'a>> {
        let full_box_header = FullBoxHeader::<StszSpec>::parse_in(cur)?;

        let sample_size = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let sample_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let entries = if sample_size == 0 {
            let expected_size = sample_count as usize * Self::ENTRY_SIZE;

            if cur.remaining() != expected_size {
                return Err(Error::at_in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Entries length does not match sample count",
                        got: cur.remaining() as u64,
                    },
                    cur.position() as u64,
                    BoxType::STSZ,
                ));
            }

            cur.take(cur.remaining())?
        } else {
            if !cur.is_empty() {
                return Err(Error::at_in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Extra data after stsz fields when sample_size is non-zero",
                        got: cur.remaining() as u64,
                    },
                    cur.position() as u64,
                    BoxType::STSZ,
                ));
            }
            &[]
        };

        Ok(StszBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            sample_size,
            sample_count,
            entries,
        })
    }

    /// Parses a `StszBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<StszBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        let this = StszBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for StszBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StszBoxView::parse(value)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for StszBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> Result<Self> {
        if value.header.boxtype() != BoxType::STSZ {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STSZ,
                found: value.header.boxtype(),
            }));
        }

        StszBoxView::parse(value.payload)
    }
}

/// Specification for the Sample Size Box (`stsz`).
pub struct StszSpec;

/// Flags for the Sample Size Box (`stsz`).
pub type StszFlags = FullBoxFlags<StszSpec>;

#[cfg(feature = "alloc")]
pub use owned::SampleSizes;
#[cfg(feature = "alloc")]
pub use owned::StszBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// Sample sizes storage for owned StszBox.
    #[derive(Debug, Clone)]
    pub enum SampleSizes {
        /// All samples have the same size.
        Uniform(u32),
        /// Each sample has its own size.
        Variable(Vec<u32>),
    }

    /// An owned Sample Size Box (`stsz`).
    #[derive(Debug, Clone)]
    pub struct StszBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: StszFlags,
        /// The number of samples in the track.
        pub sample_count: u32,
        /// The sample sizes.
        pub sample_sizes: SampleSizes,
    }

    impl StszBox {
        /// Creates a `StszBox` from a `StszBoxView`.
        pub fn from_view(view: &StszBoxView<'_>) -> Result<StszBox> {
            let sample_sizes = if view.sample_size != 0 {
                SampleSizes::Uniform(view.sample_size)
            } else {
                let sizes: Result<Vec<u32>> = view.sample_sizes().collect();
                SampleSizes::Variable(sizes?)
            };

            Ok(StszBox {
                version: view.version,
                flags: view.flags,
                sample_count: view.sample_count,
                sample_sizes,
            })
        }

        /// Parses a `StszBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<StszBox> {
            let view = StszBoxView::parse(payload)?;
            StszBox::from_view(&view)
        }

        /// Returns the size of a specific sample (1-indexed).
        pub fn get_sample_size(&self, sample_number: u32) -> Option<u32> {
            if sample_number == 0 || sample_number > self.sample_count {
                return None;
            }

            match &self.sample_sizes {
                SampleSizes::Uniform(size) => Some(*size),
                SampleSizes::Variable(sizes) => sizes.get((sample_number - 1) as usize).copied(),
            }
        }
    }

    impl TryFrom<&StszBoxView<'_>> for StszBox {
        type Error = Error;

        fn try_from(value: &StszBoxView<'_>) -> Result<Self> {
            StszBox::from_view(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_stsz_payload_uniform(sample_size: u32, sample_count: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&sample_size.to_be_bytes());
        payload.extend_from_slice(&sample_count.to_be_bytes());
        payload
    }

    fn make_stsz_payload_variable(sizes: &[u32]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&0u32.to_be_bytes()); // sample_size = 0
        payload.extend_from_slice(&(sizes.len() as u32).to_be_bytes());
        for &size in sizes {
            payload.extend_from_slice(&size.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_stsz_uniform() {
        let payload = make_stsz_payload_uniform(1024, 100);
        let stsz = StszBoxView::parse(&payload).unwrap();

        assert_eq!(stsz.version, 0);
        assert_eq!(stsz.sample_size, 1024);
        assert_eq!(stsz.sample_count, 100);

        // All samples should have the same size
        assert_eq!(stsz.get_sample_size(1).unwrap(), 1024);
        assert_eq!(stsz.get_sample_size(50).unwrap(), 1024);
        assert_eq!(stsz.get_sample_size(100).unwrap(), 1024);
    }

    #[test]
    fn parse_stsz_variable() {
        let sizes = vec![100, 200, 300, 150, 250];
        let payload = make_stsz_payload_variable(&sizes);
        let stsz = StszBoxView::parse(&payload).unwrap();

        assert_eq!(stsz.version, 0);
        assert_eq!(stsz.sample_size, 0);
        assert_eq!(stsz.sample_count, 5);

        // Each sample should have its own size
        assert_eq!(stsz.get_sample_size(1).unwrap(), 100);
        assert_eq!(stsz.get_sample_size(2).unwrap(), 200);
        assert_eq!(stsz.get_sample_size(3).unwrap(), 300);
        assert_eq!(stsz.get_sample_size(4).unwrap(), 150);
        assert_eq!(stsz.get_sample_size(5).unwrap(), 250);
    }

    #[test]
    fn parse_stsz_empty() {
        let payload = make_stsz_payload_variable(&[]);
        let stsz = StszBoxView::parse(&payload).unwrap();

        assert_eq!(stsz.sample_count, 0);
        assert_eq!(stsz.sample_sizes().count(), 0);
    }

    #[test]
    fn parse_stsz_sample_sizes_iterator_uniform() {
        let payload = make_stsz_payload_uniform(512, 3);
        let stsz = StszBoxView::parse(&payload).unwrap();

        let sizes: Vec<u32> = stsz.sample_sizes().map(|r| r.unwrap()).collect();
        assert_eq!(sizes, vec![512, 512, 512]);
    }

    #[test]
    fn parse_stsz_sample_sizes_iterator_variable() {
        let sizes = vec![100, 200, 300];
        let payload = make_stsz_payload_variable(&sizes);
        let stsz = StszBoxView::parse(&payload).unwrap();

        let parsed_sizes: Vec<u32> = stsz.sample_sizes().map(|r| r.unwrap()).collect();
        assert_eq!(parsed_sizes, sizes);
    }

    #[test]
    fn parse_stsz_sample_number_out_of_range() {
        let payload = make_stsz_payload_uniform(1024, 10);
        let stsz = StszBoxView::parse(&payload).unwrap();

        // Sample number 0 is invalid
        assert!(stsz.get_sample_size(0).is_err());
        // Sample number > sample_count is invalid
        assert!(stsz.get_sample_size(11).is_err());
    }

    #[test]
    fn parse_stsz_variable_entry_count_mismatch() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&0u32.to_be_bytes()); // sample_size = 0
        payload.extend_from_slice(&3u32.to_be_bytes()); // sample_count = 3
        payload.extend_from_slice(&100u32.to_be_bytes()); // Only 1 entry

        let result = StszBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_stsz_uniform_extra_data() {
        let mut payload = make_stsz_payload_uniform(1024, 10);
        payload.extend_from_slice(&[0, 0, 0, 0]); // Extra data

        let result = StszBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_byte_slice() {
        let payload = make_stsz_payload_uniform(512, 5);
        let stsz = StszBoxView::try_from(payload.as_slice()).unwrap();

        assert_eq!(stsz.sample_size, 512);
        assert_eq!(stsz.sample_count, 5);
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_stsz_payload_uniform(256, 10);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stsz");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let stsz = StszBoxView::try_from(&box_view).unwrap();

        assert_eq!(stsz.sample_size, 256);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_stsz_payload_uniform(256, 10);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stts"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = StszBoxView::try_from(&box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MismatchedBoxType { .. }));
        }
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn stsz_box_from_view_uniform() {
            let payload = make_stsz_payload_uniform(1024, 5);
            let view = StszBoxView::parse(&payload).unwrap();
            let stsz_box = StszBox::from_view(&view).unwrap();

            assert_eq!(stsz_box.sample_count, 5);
            assert!(matches!(stsz_box.sample_sizes, SampleSizes::Uniform(1024)));
            assert_eq!(stsz_box.get_sample_size(1), Some(1024));
            assert_eq!(stsz_box.get_sample_size(5), Some(1024));
        }

        #[test]
        fn stsz_box_from_view_variable() {
            let sizes = vec![100, 200, 300];
            let payload = make_stsz_payload_variable(&sizes);
            let view = StszBoxView::parse(&payload).unwrap();
            let stsz_box = StszBox::from_view(&view).unwrap();

            assert_eq!(stsz_box.sample_count, 3);
            if let SampleSizes::Variable(v) = &stsz_box.sample_sizes {
                assert_eq!(v, &sizes);
            } else {
                panic!("Expected Variable sample sizes");
            }
            assert_eq!(stsz_box.get_sample_size(1), Some(100));
            assert_eq!(stsz_box.get_sample_size(2), Some(200));
            assert_eq!(stsz_box.get_sample_size(3), Some(300));
        }

        #[test]
        fn stsz_box_get_sample_size_out_of_range() {
            let payload = make_stsz_payload_uniform(1024, 5);
            let stsz_box = StszBox::parse(&payload).unwrap();

            assert_eq!(stsz_box.get_sample_size(0), None);
            assert_eq!(stsz_box.get_sample_size(6), None);
        }

        #[test]
        fn stsz_box_empty() {
            let payload = make_stsz_payload_variable(&[]);
            let stsz_box = StszBox::parse(&payload).unwrap();

            assert_eq!(stsz_box.sample_count, 0);
        }
    }
}
