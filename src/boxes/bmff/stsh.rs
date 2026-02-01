use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Shadow Sync Sample Box (`stsh`).
    StshFlags {}
);

/// An entry in the Shadow Sync Sample Box (`stsh`).
#[derive(Debug, Clone, Copy)]
pub struct StshEntry {
    /// The sample number of the shadowed sample.
    pub shadowed_sample_number: u32,
    /// The sample number of the sync sample that shadows the shadowed sample.
    pub sync_sample_number: u32,
}

impl FixedSizeEntry for StshEntry {
    const ENTRY_SIZE: usize = 8;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        StshEntry {
            shadowed_sample_number: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            sync_sample_number: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.shadowed_sample_number.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.sync_sample_number.to_be_bytes());
    }
}

/// A reference to a Shadow Sync Sample Box (`stsh`).
#[derive(Debug)]
pub struct StshBoxView<'a> {
    /// Box version (0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: StshFlags,
    entries: &'a [u8],
}

impl<'a> StshBoxView<'a> {
    /// Returns an iterator over the entries in the Shadow Sync Sample Box (`stsh`).
    pub fn entries(&self) -> FixedSizeEntryIter<'a, StshEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

impl BoxCodec for StshBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSH
    }
}

impl<'de> BoxDecode<'de> for StshBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;

        // Read flags (3 bytes)
        let flags = StshFlags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;
        let expected_size = entry_count as usize * StshEntry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STSH,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(StshBoxView {
            version,
            flags,
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

    /// An owned Shadow Sync Sample Box (`stsh`).
    #[derive(Debug, Clone)]
    pub struct StshBox {
        /// Box version (0).
        pub version: u8,
        /// Box flags (should be 0).
        pub flags: StshFlags,
        /// The entries in the Shadow Sync Sample Box.
        pub entries: Vec<StshEntry>,
    }

    impl From<&StshBoxView<'_>> for StshBox {
        fn from(view: &StshBoxView<'_>) -> Self {
            Self {
                version: view.version,
                flags: view.flags,
                entries: view.entries().collect(),
            }
        }
    }

    impl StshBoxView<'_> {
        /// Converts this box view into an owned box.
        pub fn to_owned(&self) -> StshBox {
            StshBox::from(self)
        }
    }

    impl BoxCodec for StshBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSH
        }
    }

    impl BoxDecode<'_> for StshBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StshBoxView::decode(bytes)?;
            Ok(StshBox::from(&view))
        }
    }

    impl BoxEncode for StshBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
                + 4 // entry count
                + self.entries.len() * StshEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            // Write version (1 byte)
            cur.write_u8(self.version)?;

            // Write flags (3 bytes)
            cur.write_array(&self.flags.to_be_bytes())?;

            // Write entry_count (4 bytes)
            cur.write_u32_be(self.entries.len() as u32)?;

            // Write entries
            for entry in &self.entries {
                let buf = cur.take_mut(StshEntry::ENTRY_SIZE)?;
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
            0x00, 0x00, 0x00, 0x0A, // entry 1: shadowed_sample_number = 10
            0x00, 0x00, 0x00, 0x01, // entry 1: sync_sample_number = 1
            0x00, 0x00, 0x00, 0x14, // entry 2: shadowed_sample_number = 20
            0x00, 0x00, 0x00, 0x0B, // entry 2: sync_sample_number = 11
        ]
    }

    #[test]
    fn test_stsh_box_view_decode() {
        let data = raw_data();
        let stsh = StshBoxView::decode(&data).unwrap();

        assert_eq!(stsh.version, 0);
        assert_eq!(stsh.flags.bits(), 0);

        let mut entries = stsh.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.shadowed_sample_number, 10);
        assert_eq!(first.sync_sample_number, 1);
        let second = entries.next().unwrap();
        assert_eq!(second.shadowed_sample_number, 20);
        assert_eq!(second.sync_sample_number, 11);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stsh_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let stsh = StshBoxView::decode(&data).unwrap();
        assert_eq!(stsh.version, 0);
        assert_eq!(stsh.entries().count(), 0);
    }

    #[test]
    fn test_stsh_box_view_invalid_size() {
        // entry_count = 2 but only 1 entry provided
        let data: [u8; 16] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x0A, // entry 1: shadowed_sample_number = 10
            0x00, 0x00, 0x00, 0x01, // entry 1: sync_sample_number = 1
        ];

        let result = StshBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stsh_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4]; // too short

        let result = StshBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stsh_entry_round_trip() {
        let entry = StshEntry {
            shadowed_sample_number: 12345,
            sync_sample_number: 67890,
        };

        let mut bytes = [0u8; 8];
        entry.to_bytes(&mut bytes);

        let decoded = StshEntry::from_bytes(&bytes);
        assert_eq!(decoded.shadowed_sample_number, entry.shadowed_sample_number);
        assert_eq!(decoded.sync_sample_number, entry.sync_sample_number);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsh_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stsh = StshBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stsh.encoded_len()];
        stsh.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsh_box_to_owned() {
        let data = raw_data();
        let view = StshBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entries().count());
    }
}
