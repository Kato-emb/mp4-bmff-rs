use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

/// An entry in the Composition Time to Sample Box (`ctts`).
#[derive(Debug, Clone, Copy)]
pub struct CttsEntry {
    /// The number of consecutive samples with the same composition offset.
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
    const ENTRY_SIZE: usize = 8;

    /// Returns an iterator over the entries in the Composition Time to Sample Box (`ctts`).
    pub fn entries(&self) -> FixedSizeEntryIter<'a, CttsEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

/// Specification for the Composition Time to Sample Box (`ctts`).
pub struct CttsSpec;

/// Flags for the Composition Time to Sample Box (`ctts`).
pub type CttsFlags = FullBoxFlags<CttsSpec>;

impl BoxCodec for CttsBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::CTTS
    }
}

impl<'de> BoxDecode<'de> for CttsBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = CttsFlags::from_bytes(cur.read_array()?);

        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "ctts version must be 0 or 1",
                    got: version,
                },
                BoxType::CTTS,
            ));
        }

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * Self::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::CTTS,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(CttsBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for CttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        CttsBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::CttsBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use super::*;

    /// An owned Composition Time to Sample Box (`ctts`).
    pub struct CttsBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: CttsFlags,
        /// The entries in the box.
        pub entries: Vec<CttsEntry>,
    }

    impl From<&CttsBoxView<'_>> for CttsBox {
        fn from(value: &CttsBoxView<'_>) -> Self {
            let entries: Vec<CttsEntry> = value.entries().collect();
            CttsBox {
                version: value.version,
                flags: value.flags,
                entries,
            }
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
            let size = 1 // version
                + 3 // flags
                + 4 // entry_count
                + (self.entries.len() * CttsEntry::ENTRY_SIZE); // entries
            size
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.sample_count)?;
                // Version 0: unsigned, Version 1: signed
                if self.version == 0 {
                    cur.write_u32_be(entry.sample_offset as u32)?;
                } else {
                    cur.write_i32_be(entry.sample_offset)?;
                }
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

    fn make_ctts_payload_v1(entries: Vec<CttsEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0));
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry.sample_count.to_be_bytes());
            payload.extend_from_slice(&entry.sample_offset.to_be_bytes());
        }
        payload
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn ctts_box_write_and_round_trip() {
        // Multiple entries with positive and negative offsets (v1)

        use crate::BoxEncode;
        let entries = vec![
            CttsEntry {
                sample_count: 10,
                sample_offset: 100,
            },
            CttsEntry {
                sample_count: 20,
                sample_offset: -50,
            },
            CttsEntry {
                sample_count: 30,
                sample_offset: 0,
            },
        ];
        let payload = make_ctts_payload_v1(entries.clone());
        let view = CttsBoxView::decode(&payload).unwrap();
        let owned = CttsBox::try_from(&view).unwrap();

        // Write to buffer
        let mut buf = vec![0u8; 256];
        let written = owned.encode_into(&mut buf).unwrap();

        // Parse again and compare
        let reparsed = CttsBox::decode(&buf[..written]).unwrap();
        assert_eq!(reparsed.version, 1);
        assert_eq!(reparsed.entries.len(), 3);
        for (i, entry) in reparsed.entries.iter().enumerate() {
            assert_eq!(entry.sample_count, entries[i].sample_count);
            assert_eq!(entry.sample_offset, entries[i].sample_offset);
        }

        // Error case: buffer too small
        let mut small_buf = vec![0u8; 10];
        assert!(owned.encode_into(&mut small_buf).is_err());

        // Error case: entry count mismatch
        let mut bad_payload = make_full_box_header(0, 0);
        bad_payload.extend_from_slice(&2u32.to_be_bytes());
        bad_payload.extend_from_slice(&10u32.to_be_bytes());
        bad_payload.extend_from_slice(&100u32.to_be_bytes());
        assert!(CttsBoxView::decode(&bad_payload).is_err());
    }
}
