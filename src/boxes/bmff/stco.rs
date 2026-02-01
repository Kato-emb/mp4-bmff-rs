use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Chunk Offset Box (`stco`).
    StcoFlags {}
);

/// An entry in the Chunk Offset Box (`stco`).
#[derive(Debug, Clone, Copy)]
pub struct StcoEntry {
    /// The chunk offset.
    pub chunk_offset: u32,
}

impl FixedSizeEntry for StcoEntry {
    const ENTRY_SIZE: usize = 4;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        StcoEntry {
            chunk_offset: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.chunk_offset.to_be_bytes());
    }
}

/// A reference to a Chunk Offset Box (`stco`).
#[derive(Debug)]
pub struct StcoBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: StcoFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StcoBoxView<'a> {
    /// Returns an iterator over the entries in the Chunk Offset Box.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, StcoEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

impl BoxCodec for StcoBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STCO
    }
}

impl<'de> BoxDecode<'de> for StcoBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StcoFlags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;
        let expected_size = entry_count as usize * StcoEntry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "entries",
                    reason: "size does not match entry_count",
                },
                BoxType::STCO,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(StcoBoxView {
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

    /// An owned Chunk Offset Box (`stco`).
    #[derive(Debug, Clone)]
    pub struct StcoBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: StcoFlags,
        /// The entries in the box.
        pub entries: Vec<StcoEntry>,
    }

    impl From<&StcoBoxView<'_>> for StcoBox {
        fn from(view: &StcoBoxView<'_>) -> Self {
            let entries = view.entries().collect();

            StcoBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl StcoBoxView<'_> {
        /// Converts the view into an owned Chunk Offset Box.
        pub fn to_owned(&self) -> StcoBox {
            StcoBox::from(self)
        }
    }

    impl BoxCodec for StcoBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STCO
        }
    }

    impl BoxDecode<'_> for StcoBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StcoBoxView::decode(bytes)?;
            Ok(StcoBox::from(&view))
        }
    }

    impl BoxEncode for StcoBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // entry_count
            + self.entries.len() * StcoEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                let buf = cur.take_mut(StcoEntry::ENTRY_SIZE)?;
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

    fn raw_data() -> [u8; 16] {
        [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags = 0
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // entries
            0x00, 0x00, 0x10, 0x00, // chunk_offset = 4096
            0x00, 0x00, 0x20, 0x00, // chunk_offset = 8192
        ]
    }

    #[test]
    fn test_stco_box_view_decode() {
        let data = raw_data();
        let stco = StcoBoxView::decode(&data).unwrap();

        assert_eq!(stco.version, 0);
        assert_eq!(stco.entry_count, 2);

        let mut entries = stco.entries();
        assert_eq!(entries.next().unwrap().chunk_offset, 4096);
        assert_eq!(entries.next().unwrap().chunk_offset, 8192);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stco_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let stco = StcoBoxView::decode(&data).unwrap();
        assert_eq!(stco.entry_count, 0);
        assert_eq!(stco.entries().count(), 0);
    }

    #[test]
    fn test_stco_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = StcoBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stco_entry_round_trip() {
        let entry = StcoEntry { chunk_offset: 12345 };

        let mut bytes = [0u8; 4];
        entry.to_bytes(&mut bytes);

        let decoded = StcoEntry::from_bytes(&bytes);
        assert_eq!(decoded.chunk_offset, entry.chunk_offset);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stco_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stco = StcoBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stco.encoded_len()];
        stco.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stco_box_to_owned() {
        let data = raw_data();
        let view = StcoBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
