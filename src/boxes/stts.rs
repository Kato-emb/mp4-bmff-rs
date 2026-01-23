use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

/// An entry in the Decoding Time to Sample Box (`stts`).
#[derive(Debug, Clone, Copy)]
pub struct SttsEntry {
    /// The number of consecutive samples with the same duration.
    pub sample_count: u32,
    /// The duration of each sample in the group.
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

/// A reference to a Decoding Time to Sample Box (`stts`).
#[derive(Debug)]
pub struct SttsBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: SttsFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

/// Specification for the Decoding Time to Sample Box (`stts`).
pub struct SttsSpec;

/// Flags for the Decoding Time to Sample Box (`stts`).
pub type SttsFlags = FullBoxFlags<SttsSpec>;

impl<'a> SttsBoxView<'a> {
    /// Returns an iterator over the entries in the Decoding Time to Sample Box (`stts`).
    pub fn entries(&self) -> FixedSizeEntryIter<'a, SttsEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

impl<'a> TryFrom<&'a [u8]> for SttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        SttsBoxView::decode(value)
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
        let flags = SttsFlags::from_bytes(cur.read_array()?);

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

        let entries = cur.take(cur.remaining())?;

        Ok(SttsBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::SttsBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use super::*;

    /// An owned Decoding Time to Sample Box (`stts`).
    pub struct SttsBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: SttsFlags,
        /// The entries in the box.
        pub entries: Vec<SttsEntry>,
    }

    impl Default for SttsBox {
        fn default() -> Self {
            SttsBox {
                version: 0,
                flags: SttsFlags::default(),
                entries: Vec::new(),
            }
        }
    }

    impl From<&SttsBoxView<'_>> for SttsBox {
        fn from(view: &SttsBoxView<'_>) -> Self {
            let entries: Vec<SttsEntry> = view.entries().collect();
            SttsBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
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
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + 4 // entry_count
                + self.entries.len() * SttsEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.sample_count)?;
                cur.write_u32_be(entry.sample_delta)?;
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

    fn make_stts_payload(entries: Vec<SttsEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry.sample_count.to_be_bytes());
            payload.extend_from_slice(&entry.sample_delta.to_be_bytes());
        }
        payload
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn stts_box_write_and_round_trip() {
        // Multiple entries

        use crate::BoxEncode;
        let entries = vec![
            SttsEntry {
                sample_count: 100,
                sample_delta: 1000,
            },
            SttsEntry {
                sample_count: 200,
                sample_delta: 2000,
            },
            SttsEntry {
                sample_count: 300,
                sample_delta: 3000,
            },
        ];
        let payload = make_stts_payload(entries.clone());
        let view = SttsBoxView::decode(&payload).unwrap();
        let owned = SttsBox::from(&view);

        // Write to buffer
        let mut buf = vec![0u8; 256];
        let written = owned.encode_into(&mut buf).unwrap();

        // Parse again and compare
        let reparsed = SttsBox::decode(&buf[..written]).unwrap();
        assert_eq!(reparsed.entries.len(), 3);
        for (i, entry) in reparsed.entries.iter().enumerate() {
            assert_eq!(entry.sample_count, entries[i].sample_count);
            assert_eq!(entry.sample_delta, entries[i].sample_delta);
        }

        // Error case: buffer too small
        let mut small_buf = vec![0u8; 10];
        assert!(owned.encode_into(&mut small_buf).is_err());

        // Error case: entry count mismatch
        let mut bad_payload = make_full_box_header(0, 0);
        bad_payload.extend_from_slice(&2u32.to_be_bytes());
        bad_payload.extend_from_slice(&100u32.to_be_bytes());
        bad_payload.extend_from_slice(&200u32.to_be_bytes());
        assert!(SttsBoxView::decode(&bad_payload).is_err());
    }
}
