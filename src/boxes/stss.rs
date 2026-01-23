use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

/// An entry in the Sync Sample Box (`stss`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StssEntry {
    /// The sample number (1-indexed as per ISO specification).
    pub sample_number: u32,
}

impl FixedSizeEntry for StssEntry {
    const ENTRY_SIZE: usize = 4;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        StssEntry {
            sample_number: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.sample_number.to_be_bytes());
    }
}

/// A reference to a Sync Sample Box (`stss`).
///
/// This box identifies the sync (random access) samples within the stream.
#[derive(Debug)]
pub struct StssBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: StssFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StssBoxView<'a> {
    const ENTRY_SIZE: usize = 4;

    /// Returns an iterator over the entries in the Sync Sample Box.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, StssEntry> {
        FixedSizeEntryIter::new(self.entries)
    }

    /// Checks if a given sample number is a sync sample.
    ///
    /// Note: This performs a linear search through the entries.
    /// For frequent lookups, consider collecting the entries into a set.
    pub fn is_sync_sample(&self, sample_number: u32) -> Result<bool> {
        for entry in self.entries() {
            if entry.sample_number == sample_number {
                return Ok(true);
            }
            // Since sample numbers are in increasing order, we can stop early
            if entry.sample_number > sample_number {
                return Ok(false);
            }
        }
        Ok(false)
    }
}

/// Specification for the Sync Sample Box (`stss`).
pub struct StssSpec;

/// Flags for the Sync Sample Box (`stss`).
pub type StssFlags = FullBoxFlags<StssSpec>;

impl<'a> TryFrom<&'a [u8]> for StssBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StssBoxView::decode(value)
    }
}

impl BoxCodec for StssBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSS
    }
}

impl<'de> BoxDecode<'de> for StssBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StssFlags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * Self::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STSS,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(StssBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::StssBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    /// An owned Sync Sample Box (`stss`).
    #[derive(Debug, Clone, Default)]
    pub struct StssBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: StssFlags,
        /// The sync sample entries.
        pub entries: Vec<StssEntry>,
    }

    impl From<&StssBoxView<'_>> for StssBox {
        fn from(view: &StssBoxView<'_>) -> Self {
            let entries: Vec<StssEntry> = view.entries().collect();
            StssBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl StssBox {
        /// Checks if a given sample number is a sync sample.
        ///
        /// Note: This performs a binary search since sample numbers are in increasing order.
        pub fn is_sync_sample(&self, sample_number: u32) -> bool {
            self.entries
                .binary_search_by_key(&sample_number, |e| e.sample_number)
                .is_ok()
        }
    }

    impl BoxCodec for StssBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSS
        }
    }

    impl BoxDecode<'_> for StssBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StssBoxView::decode(bytes)?;
            Ok(StssBox::from(&view))
        }
    }

    impl BoxEncode for StssBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + 4 // entry_count
                + self.entries.len() * StssEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.sample_number)?;
            }

            Ok(cur.position())
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

    fn make_stss_payload(sample_numbers: Vec<u32>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(sample_numbers.len() as u32).to_be_bytes());
        for sample_number in sample_numbers {
            payload.extend_from_slice(&sample_number.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_and_iterate_samples() {
        // Empty
        let payload = make_stss_payload(vec![]);
        let stss = StssBoxView::decode(&payload).unwrap();
        assert_eq!(stss.entry_count, 0);
        assert_eq!(stss.entries().count(), 0);

        // Single
        let payload = make_stss_payload(vec![10]);
        let stss = StssBoxView::decode(&payload).unwrap();
        let entry = stss.entries().next().unwrap();
        assert_eq!(entry.sample_number, 10);

        // Multiple
        let samples = vec![1, 5, 10, 15];
        let payload = make_stss_payload(samples.clone());
        let stss = StssBoxView::decode(&payload).unwrap();
        let parsed: Vec<_> = stss.entries().map(|r| r.sample_number).collect();
        assert_eq!(parsed, samples);
    }

    #[test]
    fn invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        assert!(StssBoxView::decode(&payload).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let samples = vec![1, 5, 10];
        let payload = make_stss_payload(samples.clone());
        let view = StssBoxView::decode(&payload).unwrap();
        let owned = StssBox::from(&view);
        assert_eq!(owned.entries.len(), 3);
        assert_eq!(owned.entries[0].sample_number, 1);
        assert!(owned.is_sync_sample(1));
        assert!(!owned.is_sync_sample(2));
    }
}
