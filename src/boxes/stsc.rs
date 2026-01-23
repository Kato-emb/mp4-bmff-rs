use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

/// An entry in the Sample To Chunk Box (`stsc`).
#[derive(Debug, Clone, Copy)]
pub struct StscEntry {
    /// The first chunk number.
    pub first_chunk: u32,
    /// The samples per chunk.
    pub samples_per_chunk: u32,
    /// The sample description index.
    pub sample_description_index: u32,
}

impl FixedSizeEntry for StscEntry {
    const ENTRY_SIZE: usize = 12;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        StscEntry {
            first_chunk: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            samples_per_chunk: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            sample_description_index: u32::from_be_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11],
            ]),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.first_chunk.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.samples_per_chunk.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.sample_description_index.to_be_bytes());
    }
}

/// A reference to a Sample To Chunk Box (`stsc`).
#[derive(Debug)]
pub struct StscBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: StscFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StscBoxView<'a> {
    const ENTRY_SIZE: usize = 12;

    /// Returns an iterator over the entries in the Sample To Chunk Box.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, StscEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

/// Specification for the Sample To Chunk Box (`stsc`).
pub struct StscSpec;

/// Flags for the Sample To Chunk Box (`stsc`).
pub type StscFlags = FullBoxFlags<StscSpec>;

impl<'a> TryFrom<&'a [u8]> for StscBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StscBoxView::decode(value)
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
        let flags = StscFlags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * Self::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STSC,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(StscBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::StscBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use super::*;

    /// An owned Sample To Chunk Box (`stsc`).
    #[derive(Debug, Clone)]
    pub struct StscBox {
        /// The version of the Sample To Chunk Box.
        pub version: u8,
        /// The flags of the Sample To Chunk Box.
        pub flags: StscFlags,
        /// The entries in the Sample To Chunk Box.
        pub entries: Vec<StscEntry>,
    }

    impl Default for StscBox {
        fn default() -> Self {
            StscBox {
                version: 0,
                flags: StscFlags::default(),
                entries: Vec::new(),
            }
        }
    }

    impl From<&StscBoxView<'_>> for StscBox {
        fn from(view: &StscBoxView<'_>) -> Self {
            let entries = view.entries().collect::<Vec<StscEntry>>();

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
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + 4 // entry_count
                + self.entries.len() * StscEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.first_chunk)?;
                cur.write_u32_be(entry.samples_per_chunk)?;
                cur.write_u32_be(entry.sample_description_index)?;
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

    fn make_stsc_payload(entries: Vec<StscEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry.first_chunk.to_be_bytes());
            payload.extend_from_slice(&entry.samples_per_chunk.to_be_bytes());
            payload.extend_from_slice(&entry.sample_description_index.to_be_bytes());
        }
        payload
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn stsc_box_write_and_round_trip() {
        // Multiple entries

        use crate::BoxEncode;
        let entries = vec![
            StscEntry {
                first_chunk: 1,
                samples_per_chunk: 10,
                sample_description_index: 1,
            },
            StscEntry {
                first_chunk: 5,
                samples_per_chunk: 20,
                sample_description_index: 2,
            },
            StscEntry {
                first_chunk: 10,
                samples_per_chunk: 15,
                sample_description_index: 1,
            },
        ];
        let payload = make_stsc_payload(entries.clone());
        let view = StscBoxView::decode(&payload).unwrap();
        let owned = StscBox::from(&view);

        // Write to buffer
        let mut buf = vec![0u8; 256];
        let written = owned.encode_into(&mut buf).unwrap();

        // Parse again and compare
        let reparsed = StscBox::decode(&buf[..written]).unwrap();
        assert_eq!(reparsed.entries.len(), 3);
        for (i, entry) in reparsed.entries.iter().enumerate() {
            assert_eq!(entry.first_chunk, entries[i].first_chunk);
            assert_eq!(entry.samples_per_chunk, entries[i].samples_per_chunk);
            assert_eq!(
                entry.sample_description_index,
                entries[i].sample_description_index
            );
        }

        // Error case: buffer too small
        let mut small_buf = vec![0u8; 10];
        assert!(owned.encode_into(&mut small_buf).is_err());

        // Error case: entry count mismatch
        let mut bad_payload = make_full_box_header(0, 0);
        bad_payload.extend_from_slice(&2u32.to_be_bytes());
        bad_payload.extend_from_slice(&1u32.to_be_bytes());
        bad_payload.extend_from_slice(&10u32.to_be_bytes());
        bad_payload.extend_from_slice(&1u32.to_be_bytes());
        assert!(StscBoxView::decode(&bad_payload).is_err());
    }
}
