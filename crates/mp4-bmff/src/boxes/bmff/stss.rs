//! Sync Sample Box (`stss`) implementation.
//!
//! The Sync Sample Box identifies the sync samples (random access points)
//! within the track. For video, these are typically I-frames/keyframes.
//! If this box is absent, every sample is considered a sync sample.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedEntry;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Sync Sample Box (`stss`).
    ///
    /// Reserved (should be 0).
    StssFlags {}
);

/// An entry in the Sync Sample Box (`stss`).
///
/// Identifies a single sync sample (random access point) by its
/// 1-based sample number.
#[derive(Debug, Clone, Copy)]
pub struct StssEntry {
    /// The sample number of a sync sample.
    pub sample_number: u32,
}

impl FixedEntry<4> for StssEntry {
    fn from_bytes(bytes: &[u8; 4]) -> Self {
        StssEntry {
            sample_number: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        }
    }

    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> [u8; 4] {
        self.sample_number.to_be_bytes()
    }
}

define_entry_iter!(
    /// An iterator over entries in the Sync Sample Box (`stss`).
    pub struct StssEntryIter(StssEntry, 4);
);

/// A reference to a Sync Sample Box (`stss`).
///
/// Lists all sync samples (random access points/keyframes) in the track.
/// Used for seeking to specific points in the media.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `entry_count`: Number of sync samples.
/// - `entries`: Array of 1-based sample numbers that are sync samples.
#[derive(Debug)]
pub struct StssBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: StssFlags,
    /// Number of sync samples in the track.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StssBoxView<'a> {
    /// Returns an iterator over the entries in the Sync Sample Box (`stss`).
    pub fn entries(&self) -> StssEntryIter<'a> {
        StssEntryIter::new(self.entries)
    }
}

impl BoxCodec for StssBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSS
    }
}

impl<'de> BoxDecode<'de> for StssBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StssFlags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * StssEntry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STSS,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(StssBoxView {
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

    /// An owned Sync Sample Box (`stss`).
    ///
    /// This is the owned variant of [`StssBoxView`] that stores sync sample
    /// numbers in a heap-allocated vector.
    ///
    /// # Structure
    ///
    /// - `version`: Box version (should be 0).
    /// - `flags`: Reserved (should be 0).
    /// - `entries`: Sync sample numbers (1-based).
    #[derive(Debug, Clone)]
    pub struct StssBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: StssFlags,
        /// Sample numbers of sync samples (keyframes).
        pub entries: Vec<StssEntry>,
    }

    impl From<&StssBoxView<'_>> for StssBox {
        fn from(view: &StssBoxView<'_>) -> Self {
            let entries = view.entries().collect();

            StssBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl StssBoxView<'_> {
        /// Converts this view into an owned `StssBox`.
        pub fn to_owned(&self) -> StssBox {
            StssBox::from(self)
        }
    }

    impl BoxCodec for StssBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSS
        }
    }

    impl BoxDecode<'_> for StssBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StssBoxView::decode(bytes)?;
            Ok(StssBox::from(&view))
        }
    }

    impl BoxEncode for StssBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // entry_count
            + self.entries.len() * StssEntry::ENTRY_SIZE // entries
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

    fn raw_data() -> [u8; 16] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x01, // entry 1: sample_number = 1
            0x00, 0x00, 0x00, 0x64, // entry 2: sample_number = 100
        ]
    }

    #[test]
    fn test_stss_box_view_decode() {
        let data = raw_data();
        let stss = StssBoxView::decode(&data).unwrap();

        assert_eq!(stss.version, 0);
        assert_eq!(stss.flags.bits(), 0);
        assert_eq!(stss.entry_count, 2);

        let mut entries = stss.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.sample_number, 1);
        let second = entries.next().unwrap();
        assert_eq!(second.sample_number, 100);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stss_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let stss = StssBoxView::decode(&data).unwrap();
        assert_eq!(stss.version, 0);
        assert_eq!(stss.entry_count, 0);
        assert_eq!(stss.entries().count(), 0);
    }

    #[test]
    fn test_stss_box_view_invalid_size() {
        // entry_count = 2 but only 1 entry provided
        let data: [u8; 12] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x01, // entry 1: sample_number = 1
        ];

        let result = StssBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stss_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4]; // too short

        let result = StssBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stss_entry_round_trip() {
        let entry = StssEntry {
            sample_number: 12345,
        };

        let bytes = entry.to_bytes();

        let decoded = StssEntry::from_bytes(&bytes);
        assert_eq!(decoded.sample_number, entry.sample_number);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stss_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stss = StssBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stss.encoded_len()];
        stss.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stss_box_to_owned() {
        let data = raw_data();
        let view = StssBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
