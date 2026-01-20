use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

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
        let entry_size = cursor.read_u32_be()?;

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
        let version = cur.read_u8()?;
        let flags = StszFlags::from_bytes(cur.read_array()?);

        let sample_size = cur.read_u32_be()?;

        let sample_count = cur.read_u32_be()?;

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
            version,
            flags,
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

impl<'a> TryFrom<BoxFrame<'a>> for StszBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::STSZ {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STSZ,
                found: value.boxtype(),
            }));
        }

        StszBoxView::parse(value.payload())
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
    use crate::cursor::WriteCursor;

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
        const ENTRY_SIZE: usize = 4;

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

        /// Returns the size of the `StszBox` data.
        #[inline]
        pub fn size(&self) -> usize {
            // version/flags + sample_size + sample_count + entries (if variable)
            4 + 4
                + 4
                + match &self.sample_sizes {
                    SampleSizes::Uniform(_) => 0,
                    SampleSizes::Variable(sizes) => sizes.len() * Self::ENTRY_SIZE,
                }
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;

            match &self.sample_sizes {
                SampleSizes::Uniform(size) => {
                    cur.write_u32_be(*size)?;
                    cur.write_u32_be(self.sample_count)?;
                }
                SampleSizes::Variable(sizes) => {
                    cur.write_u32_be(0)?;
                    cur.write_u32_be(sizes.len() as u32)?;
                    for size in sizes {
                        cur.write_u32_be(*size)?;
                    }
                }
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::STSZ,
                ));
            }

            Ok(())
        }

        /// Writes this `StszBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
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

    fn make_stsz_payload_variable(sample_sizes: Vec<u32>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&0u32.to_be_bytes());
        payload.extend_from_slice(&(sample_sizes.len() as u32).to_be_bytes());
        for size in sample_sizes {
            payload.extend_from_slice(&size.to_be_bytes());
        }
        payload
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn stsz_box_write_and_round_trip() {
        // Variable sizes case
        let sizes = vec![100, 200, 300, 150, 250];
        let payload = make_stsz_payload_variable(sizes.clone());
        let view = StszBoxView::parse(&payload).unwrap();
        let owned = StszBox::from_view(&view).unwrap();

        // Write to buffer
        let mut buf = vec![0u8; owned.size()];
        owned.write(&mut buf).unwrap();

        // Parse again and compare
        let reparsed = StszBox::parse(&buf).unwrap();
        if let SampleSizes::Variable(ref s) = reparsed.sample_sizes {
            assert_eq!(s, &sizes);
        } else {
            panic!("Expected Variable sample sizes");
        }

        // Uniform sizes case
        let payload = make_stsz_payload_uniform(1024, 10);
        let view = StszBoxView::parse(&payload).unwrap();
        let owned = StszBox::from_view(&view).unwrap();

        let mut buf = vec![0u8; owned.size()];
        owned.write(&mut buf).unwrap();

        let reparsed = StszBox::parse(&buf).unwrap();
        assert_eq!(reparsed.sample_count, 10);
        if let SampleSizes::Uniform(size) = reparsed.sample_sizes {
            assert_eq!(size, 1024);
        } else {
            panic!("Expected Uniform sample sizes");
        }

        // Error case: buffer too small
        let mut small_buf = vec![0u8; owned.size() - 1];
        assert!(owned.write(&mut small_buf).is_err());

        // Error case: entry count mismatch
        let mut bad_payload = make_full_box_header(0, 0);
        bad_payload.extend_from_slice(&0u32.to_be_bytes());
        bad_payload.extend_from_slice(&2u32.to_be_bytes());
        bad_payload.extend_from_slice(&100u32.to_be_bytes());
        assert!(StszBoxView::parse(&bad_payload).is_err());
    }
}
