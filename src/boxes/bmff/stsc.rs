use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Sample to Chunk Box (`stsc`).
    StscFlags {}
);

/// An entry in the Sample to Chunk Box (`stsc`).
#[derive(Debug, Clone, Copy)]
pub struct StscEntry {
    /// The index of the first chunk in this entry (1-based).
    pub first_chunk: u32,
    /// The number of samples per chunk.
    pub samples_per_chunk: u32,
    /// The sample description index for samples in this chunk.
    pub sample_description_index: u32,
}

impl FixedSizeEntry for StscEntry {
    const ENTRY_SIZE: usize = 12;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        let first_chunk = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let samples_per_chunk = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let sample_description_index =
            u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        StscEntry {
            first_chunk,
            samples_per_chunk,
            sample_description_index,
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.first_chunk.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.samples_per_chunk.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.sample_description_index.to_be_bytes());
    }
}

/// A reference to a Sample to Chunk Box (`stsc`).
#[derive(Debug)]
pub struct StscBoxView<'a> {
    /// The version of the box (0).
    pub version: u8,
    /// The flags of the box.
    pub flags: StscFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StscBoxView<'a> {
    /// Returns an iterator over the entries in the Sample to Chunk Box (`stsc`).
    pub fn entries(&self) -> FixedSizeEntryIter<'a, StscEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

impl BoxCodec for StscBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSC
    }
}

impl<'de> BoxDecode<'de> for StscBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StscFlags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * StscEntry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "entries",
                    reason: "size does not match entry_count",
                },
                BoxType::STSC,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(StscBoxView {
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

    /// An owned Sample to Chunk Box (`stsc`).
    #[derive(Debug, Clone)]
    pub struct StscBox {
        /// The version of the box (0).
        pub version: u8,
        /// The flags of the box.
        pub flags: StscFlags,
        /// The entries in the box.
        pub entries: Vec<StscEntry>,
    }

    impl From<&StscBoxView<'_>> for StscBox {
        fn from(view: &StscBoxView<'_>) -> Self {
            let entries = view.entries().collect();
            StscBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl BoxCodec for StscBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSC
        }
    }

    impl BoxDecode<'_> for StscBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StscBoxView::decode(bytes)?;
            Ok(StscBox::from(&view))
        }
    }

    impl BoxEncode for StscBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // entry_count
            + self.entries.len() * StscEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                let buf = cur.take_mut(StscEntry::ENTRY_SIZE)?;
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

    fn raw_data() -> [u8; 20] {
        [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags = 0
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // entry (12 bytes)
            0x00, 0x00, 0x00, 0x01, // first_chunk = 1
            0x00, 0x00, 0x00, 0x0A, // samples_per_chunk = 10
            0x00, 0x00, 0x00, 0x01, // sample_description_index = 1
        ]
    }

    #[test]
    fn test_stsc_box_view_decode() {
        let data = raw_data();
        let stsc = StscBoxView::decode(&data).unwrap();

        assert_eq!(stsc.version, 0);
        assert_eq!(stsc.entry_count, 1);

        let mut entries = stsc.entries();
        let entry = entries.next().unwrap();
        assert_eq!(entry.first_chunk, 1);
        assert_eq!(entry.samples_per_chunk, 10);
        assert_eq!(entry.sample_description_index, 1);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stsc_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let stsc = StscBoxView::decode(&data).unwrap();
        assert_eq!(stsc.entry_count, 0);
        assert_eq!(stsc.entries().count(), 0);
    }

    #[test]
    fn test_stsc_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = StscBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stsc_entry_round_trip() {
        let entry = StscEntry {
            first_chunk: 5,
            samples_per_chunk: 20,
            sample_description_index: 1,
        };

        let mut bytes = [0u8; 12];
        entry.to_bytes(&mut bytes);

        let decoded = StscEntry::from_bytes(&bytes);
        assert_eq!(decoded.first_chunk, entry.first_chunk);
        assert_eq!(decoded.samples_per_chunk, entry.samples_per_chunk);
        assert_eq!(decoded.sample_description_index, entry.sample_description_index);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsc_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stsc = StscBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stsc.encoded_len()];
        stsc.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsc_box_from_view() {
        let data = raw_data();
        let view = StscBoxView::decode(&data).unwrap();
        let owned = StscBox::from(&view);

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
