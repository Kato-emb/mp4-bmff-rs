use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

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
    const ENTRY_SIZE: usize = 4;

    /// Returns an iterator over the entries in the Chunk Offset Box.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, StcoEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

/// Specification for the Chunk Offset Box (`stco`).
pub struct StcoSpec;

/// Flags for the Chunk Offset Box (`stco`).
pub type StcoFlags = FullBoxFlags<StcoSpec>;

impl BoxCodec for StcoBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STCO
    }
}

impl<'de> BoxDecode<'de> for StcoBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StcoFlags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * Self::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STCO,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(StcoBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for StcoBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StcoBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::StcoBox;

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
            let entries: Vec<StcoEntry> = view.entries().collect();
            StcoBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
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
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + 4 // entry_count
                + self.entries.len() * StcoEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.chunk_offset)?;
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

    fn make_stco_payload(offsets: Vec<u32>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(offsets.len() as u32).to_be_bytes());
        for offset in offsets {
            payload.extend_from_slice(&offset.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_and_iterate_offsets() {
        // Empty
        let payload = make_stco_payload(vec![]);
        let stco = StcoBoxView::decode(&payload).unwrap();
        assert_eq!(stco.entry_count, 0);
        assert_eq!(stco.entries().count(), 0);

        // Single
        let payload = make_stco_payload(vec![1000]);
        let stco = StcoBoxView::decode(&payload).unwrap();
        assert_eq!(stco.entry_count, 1);
        assert_eq!(stco.entries().next().unwrap().chunk_offset, 1000);

        // Multiple
        let offsets = vec![100, 200, 300, 400];
        let payload = make_stco_payload(offsets.clone());
        let stco = StcoBoxView::decode(&payload).unwrap();
        let parsed: Vec<_> = stco.entries().map(|e| e.chunk_offset).collect();
        assert_eq!(parsed, offsets);
    }

    #[test]
    fn invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&100u32.to_be_bytes());
        assert!(StcoBoxView::decode(&payload).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let offsets = vec![100, 200, 300];
        let payload = make_stco_payload(offsets.clone());
        let view = StcoBoxView::decode(&payload).unwrap();
        let owned = StcoBox::try_from(&view).unwrap();
        assert_eq!(owned.entries.len(), 3);
        assert_eq!(owned.entries[0].chunk_offset, 100);
    }
}
