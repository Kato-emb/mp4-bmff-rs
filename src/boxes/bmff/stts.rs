use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Decoding Time to Sample Box ('stts')
    SttsFlags {}
);

/// An entry in the Decoding Time to Sample Box ('stts').
#[derive(Debug, Clone, Copy)]
pub struct SttsEntry {
    /// The number of consecutive samples having the same duration.
    pub sample_count: u32,
    /// The duration of each sample, in time units.
    pub sample_delta: u32,
}

impl FixedSizeEntry for SttsEntry {
    const ENTRY_SIZE: usize = 8;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        SttsEntry {
            sample_count: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            sample_delta: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.sample_count.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.sample_delta.to_be_bytes());
    }
}

/// A reference to a Decoding Time to Sample Box ('stts').
#[derive(Debug)]
pub struct SttsBoxView<'a> {
    /// Box version.
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: SttsFlags,
    /// Number of entries.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> SttsBoxView<'a> {
    /// Returns an iterator over the STTS entries.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, SttsEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

impl BoxCodec for SttsBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STTS
    }
}

impl<'de> BoxDecode<'de> for SttsBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = SttsFlags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * SttsEntry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STTS,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(SttsBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Decoding Time to Sample Box ('stts').
    #[derive(Debug, Clone)]
    pub struct SttsBox {
        /// Box version.
        pub version: u8,
        /// Box flags (should be 0).
        pub flags: SttsFlags,
        /// STTS entries.
        pub entries: Vec<SttsEntry>,
    }

    impl From<&SttsBoxView<'_>> for SttsBox {
        fn from(view: &SttsBoxView<'_>) -> Self {
            let entries = view.entries().collect();

            SttsBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl SttsBoxView<'_> {
        /// Converts this view into an owned `SttsBox`.
        pub fn to_owned(&self) -> SttsBox {
            SttsBox::from(self)
        }
    }

    impl BoxCodec for SttsBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STTS
        }
    }

    impl BoxDecode<'_> for SttsBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = SttsBoxView::decode(bytes)?;
            Ok(SttsBox::from(&view))
        }
    }

    impl BoxEncode for SttsBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // entry_count
            + self.entries.len() * SttsEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                let buf = cur.take_mut(SttsEntry::ENTRY_SIZE)?;
                entry.to_bytes(buf);
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

    fn raw_data() -> [u8; 24] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x0A, // entry 1: sample_count = 10
            0x00, 0x00, 0x03, 0xE8, // entry 1: sample_delta = 1000
            0x00, 0x00, 0x00, 0x14, // entry 2: sample_count = 20
            0x00, 0x00, 0x07, 0xD0, // entry 2: sample_delta = 2000
        ]
    }

    #[test]
    fn test_stts_box_view_decode() {
        let data = raw_data();
        let stts = SttsBoxView::decode(&data).unwrap();

        assert_eq!(stts.version, 0);
        assert_eq!(stts.flags.bits(), 0);
        assert_eq!(stts.entry_count, 2);

        let mut entries = stts.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.sample_count, 10);
        assert_eq!(first.sample_delta, 1000);
        let second = entries.next().unwrap();
        assert_eq!(second.sample_count, 20);
        assert_eq!(second.sample_delta, 2000);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stts_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let stts = SttsBoxView::decode(&data).unwrap();
        assert_eq!(stts.version, 0);
        assert_eq!(stts.entry_count, 0);
        assert_eq!(stts.entries().count(), 0);
    }

    #[test]
    fn test_stts_box_view_invalid_size() {
        // entry_count = 2 but only 1 entry provided
        let data: [u8; 16] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x0A, // entry 1: sample_count = 10
            0x00, 0x00, 0x03, 0xE8, // entry 1: sample_delta = 1000
        ];

        let result = SttsBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stts_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4]; // too short

        let result = SttsBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stts_entry_round_trip() {
        let entry = SttsEntry {
            sample_count: 12345,
            sample_delta: 67890,
        };

        let mut bytes = [0u8; 8];
        entry.to_bytes(&mut bytes);

        let decoded = SttsEntry::from_bytes(&bytes);
        assert_eq!(decoded.sample_count, entry.sample_count);
        assert_eq!(decoded.sample_delta, entry.sample_delta);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stts_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stts = SttsBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stts.encoded_len()];
        stts.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stts_box_to_owned() {
        let data = raw_data();
        let view = SttsBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
