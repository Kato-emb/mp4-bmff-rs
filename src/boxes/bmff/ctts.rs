use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Composition Time to Sample Box (`ctts`).
    CttsFlags {}
);

/// An entry in the Composition Time to Sample Box (`ctts`).
#[derive(Debug, Clone, Copy)]
pub struct CttsEntry {
    /// The number of consecutive samples with the same offset.
    pub sample_count: u32,
    /// The composition offset for each sample in the group.
    /// This is a signed value to handle version 1 negative offsets.
    pub sample_offset: i32,
}

impl FixedSizeEntry for CttsEntry {
    const ENTRY_SIZE: usize = 8;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        let sample_count = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let sample_offset = i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        CttsEntry {
            sample_count,
            sample_offset,
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.sample_count.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.sample_offset.to_be_bytes());
    }
}

/// A reference to a Composition Time to Sample Box (`ctts`).
#[derive(Debug)]
pub struct CttsBoxView<'a> {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: CttsFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> CttsBoxView<'a> {
    /// Returns an iterator over the entries in the Composition Time to Sample Box (`ctts`).
    pub fn entries(&self) -> FixedSizeEntryIter<'a, CttsEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

impl BoxCodec for CttsBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::CTTS
    }
}

impl<'de> BoxDecode<'de> for CttsBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;

        // Read flags (3 bytes)
        let flags = CttsFlags::from_be_bytes(cur.read_array::<3>()?);

        // Read entry_count (4 bytes)
        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * CttsEntry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "entries",
                    reason: "size does not match entry_count",
                },
                BoxType::CTTS,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(CttsBoxView {
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

    /// An owned Composition Time to Sample Box (`ctts`).
    #[derive(Debug, Clone)]
    pub struct CttsBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: CttsFlags,
        /// The entries in the box.
        pub entries: Vec<CttsEntry>,
    }

    impl From<&CttsBoxView<'_>> for CttsBox {
        fn from(view: &CttsBoxView<'_>) -> Self {
            let entries = view.entries().collect();
            CttsBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl CttsBoxView<'_> {
        /// Converts this view into an owned `CttsBox`.
        pub fn to_owned(&self) -> CttsBox {
            CttsBox::from(self)
        }
    }

    impl BoxCodec for CttsBox {
        fn boxtype(&self) -> BoxType {
            BoxType::CTTS
        }
    }

    impl BoxDecode<'_> for CttsBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = CttsBoxView::decode(bytes)?;
            Ok(CttsBox::from(&view))
        }
    }

    impl BoxEncode for CttsBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + 4 // entry_count
                + self.entries.len() * CttsEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                let buf = cur.take_mut(CttsEntry::ENTRY_SIZE)?;
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
            0x01, // version = 1 (supports negative offsets)
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x0A, // entry 1: sample_count = 10
            0x00, 0x00, 0x03, 0xE8, // entry 1: sample_offset = 1000
            0x00, 0x00, 0x00, 0x14, // entry 2: sample_count = 20
            0xFF, 0xFF, 0xFC, 0x18, // entry 2: sample_offset = -1000
        ]
    }

    #[test]
    fn test_ctts_box_view_decode() {
        let data = raw_data();
        let ctts = CttsBoxView::decode(&data).unwrap();

        assert_eq!(ctts.version, 1);
        assert_eq!(ctts.flags.bits(), 0);
        assert_eq!(ctts.entry_count, 2);

        let mut entries = ctts.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.sample_count, 10);
        assert_eq!(first.sample_offset, 1000);
        let second = entries.next().unwrap();
        assert_eq!(second.sample_count, 20);
        assert_eq!(second.sample_offset, -1000);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_ctts_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let ctts = CttsBoxView::decode(&data).unwrap();
        assert_eq!(ctts.version, 0);
        assert_eq!(ctts.entry_count, 0);
        assert_eq!(ctts.entries().count(), 0);
    }

    #[test]
    fn test_ctts_box_view_invalid_size() {
        // entry_count = 2 but only 1 entry provided
        let data: [u8; 16] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x0A, // entry 1: sample_count = 10
            0x00, 0x00, 0x03, 0xE8, // entry 1: sample_offset = 1000
        ];

        let result = CttsBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_ctts_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4]; // too short

        let result = CttsBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_ctts_entry_round_trip() {
        let entry = CttsEntry {
            sample_count: 12345,
            sample_offset: -67890,
        };

        let mut bytes = [0u8; 8];
        entry.to_bytes(&mut bytes);

        let decoded = CttsEntry::from_bytes(&bytes);
        assert_eq!(decoded.sample_count, entry.sample_count);
        assert_eq!(decoded.sample_offset, entry.sample_offset);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_ctts_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let ctts = CttsBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; ctts.encoded_len()];
        ctts.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_ctts_box_to_owned() {
        let data = raw_data();
        let view = CttsBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
