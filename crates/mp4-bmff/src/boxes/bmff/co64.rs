//! Chunk Large Offset Box (`co64`) implementation.
//!
//! The Chunk Large Offset Box provides the file offset of each chunk within the
//! media data, using 64-bit offsets. Use `stco` for files smaller than
//! 4GB.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedEntry;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Chunk Large Offset Box (`co64`).
    ///
    /// Reserved (should be 0).
    Co64Flags {}
);

/// An entry in the Chunk Large Offset Box (`co64`).
///
/// Contains the 64-bit file offset of a single chunk.
#[derive(Debug, Clone, Copy)]
pub struct Co64Entry {
    /// The chunk offset.
    pub chunk_offset: u64,
}

impl FixedEntry<8> for Co64Entry {
    fn from_bytes(bytes: &[u8; 8]) -> Self {
        Co64Entry {
            chunk_offset: u64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
        }
    }

    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> [u8; 8] {
        self.chunk_offset.to_be_bytes()
    }
}

define_entry_iter!(
    /// An iterator over entries in the Chunk Large Offset Box (`co64`).
    pub struct Co64EntryIter(Co64Entry, 8);
);

/// A reference to a Chunk Large Offset Box (`co64`).
///
/// Provides the file position of each chunk (64-bit offsets). For files
/// smaller than 4GB, use `stco` instead.
///
/// # Structure
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `entry_count`: Number of chunks.
/// - `entries`: Array of 64-bit chunk offsets.
#[derive(Debug)]
pub struct Co64BoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved (should be 0).
    pub flags: Co64Flags,
    /// Number of chunks.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> Co64BoxView<'a> {
    /// Returns an iterator over the entries in the Chunk Large Offset Box.
    pub fn entries(&self) -> Co64EntryIter<'a> {
        Co64EntryIter::new(self.entries)
    }
}

impl BoxCodec for Co64BoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::CO64
    }
}

impl<'de> BoxDecode<'de> for Co64BoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = Co64Flags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;
        let expected_size = entry_count as usize * Co64Entry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "entries",
                    reason: "size does not match entry_count",
                },
                BoxType::CO64,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(Co64BoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Chunk Large Offset Box (`co64`).
    #[derive(Debug, Clone, Default)]
    pub struct Co64Box {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved (should be 0).
        pub flags: Co64Flags,
        /// Array of 64-bit chunk offsets.
        pub entries: Vec<Co64Entry>,
    }

    impl From<&Co64BoxView<'_>> for Co64Box {
        fn from(view: &Co64BoxView<'_>) -> Self {
            let entries = view.entries().collect();

            Co64Box {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl Co64BoxView<'_> {
        /// Converts this `Co64BoxView` into an owned `Co64Box`.
        pub fn to_owned(&self) -> Co64Box {
            Co64Box::from(self)
        }
    }

    impl BoxCodec for Co64Box {
        fn boxtype(&self) -> BoxType {
            BoxType::CO64
        }
    }

    impl BoxDecode<'_> for Co64Box {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = Co64BoxView::decode(bytes)?;
            Ok(Co64Box::from(&view))
        }
    }

    impl BoxEncode for Co64Box {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // entry_count
            + self.entries.len() * Co64Entry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(u32::try_from(self.entries.len())?)?;

            for entry in &self.entries {
                let bytes = entry.to_bytes();
                cur.write_array(&bytes)?;
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
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // entries
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x10,
            0x00, // chunk_offset = 4294971392 (> 4GB)
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x20,
            0x00, // chunk_offset = 8589942784 (> 4GB)
        ]
    }

    #[test]
    fn test_co64_box_view_decode() {
        let data = raw_data();
        let co64 = Co64BoxView::decode(&data).unwrap();

        assert_eq!(co64.version, 0);
        assert_eq!(co64.entry_count, 2);

        let mut entries = co64.entries();
        assert_eq!(entries.next().unwrap().chunk_offset, 0x0000_0001_0000_1000);
        assert_eq!(entries.next().unwrap().chunk_offset, 0x0000_0002_0000_2000);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_co64_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let co64 = Co64BoxView::decode(&data).unwrap();
        assert_eq!(co64.entry_count, 0);
        assert_eq!(co64.entries().count(), 0);
    }

    #[test]
    fn test_co64_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = Co64BoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_co64_entry_round_trip() {
        let entry = Co64Entry {
            chunk_offset: 0x0000_0001_0000_1000,
        };

        let bytes = entry.to_bytes();

        let decoded = Co64Entry::from_bytes(&bytes);
        assert_eq!(decoded.chunk_offset, entry.chunk_offset);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_co64_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let co64 = Co64Box::decode(&original).unwrap();

        let mut encoded = vec![0u8; co64.encoded_len()];
        co64.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_co64_box_to_owned() {
        let data = raw_data();
        let view = Co64BoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
