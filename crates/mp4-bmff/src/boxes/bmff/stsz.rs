//! Sample Size Box (`stsz`) implementation.
//!
//! The Sample Size Box contains the size of every sample in the track.
//! If all samples have the same size, a default size can be specified
//! to avoid storing individual sizes. Otherwise, each sample's size
//! is stored in the entries table.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedEntry;

define_box_flags!(
    /// Flags for the Sample Size Box (`stsz`).
    ///
    /// Reserved (should be 0).
    StszFlags {}
);

/// An entry in the Sample Size Box (`stsz`).
///
/// Represents the size of a single sample in bytes.
#[derive(Debug, Clone, Copy)]
pub struct StszEntry {
    /// The size of the sample in bytes.
    pub entry_size: u32,
}

impl FixedEntry<4> for StszEntry {
    fn from_bytes(bytes: &[u8; 4]) -> Self {
        StszEntry {
            entry_size: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        }
    }

    fn to_bytes(&self) -> [u8; 4] {
        self.entry_size.to_be_bytes()
    }
}

define_entry_iter!(
    /// An iterator over entries in the Sample Size Box (`stsz`).
    pub struct StszEntryIter(StszEntry, 4);
);

/// A reference to a Sample Size Box (`stsz`).
///
/// Provides the size of each sample in the track. If `sample_size` is non-zero,
/// all samples have that size and the entries table is empty.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `sample_size`: Default size if all samples are equal, 0 otherwise.
/// - `sample_count`: Total number of samples.
/// - `entries`: Individual sample sizes (only when `sample_size` is 0).
#[derive(Debug)]
pub struct StszBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: StszFlags,
    /// Default sample size if uniform, 0 for variable sizes.
    pub sample_size: u32,
    /// Total number of samples in the track.
    pub sample_count: u32,
    entries: &'a [u8],
}

impl<'a> StszBoxView<'a> {
    /// Returns an iterator over the entries in the Sample Size Box (`stsz`).
    pub fn entries(&self) -> StszEntryIter<'a> {
        StszEntryIter::new(self.entries)
    }
}

impl BoxCodec for StszBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSZ
    }
}

impl<'de> BoxDecode<'de> for StszBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = crate::cursor::ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StszFlags::from_be_bytes(cur.read_array::<3>()?);

        let sample_size = cur.read_u32_be()?;
        let sample_count = cur.read_u32_be()?;

        let expected_size = sample_count as usize * 4; // Each entry is 4 bytes

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "entries",
                    reason: "size does not match entry_count",
                },
                BoxType::STSZ,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(StszBoxView {
            version,
            flags,
            sample_size,
            sample_count,
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

    /// An owned Sample Size Box (`stsz`).
    ///
    /// This is the owned variant of [`StszBoxView`] that stores sample sizes
    /// in a heap-allocated vector.
    ///
    /// # Structure
    ///
    /// - `version`: Box version (should be 0).
    /// - `flags`: Reserved (should be 0).
    /// - `sample_size`: Default size if uniform, 0 for variable sizes.
    /// - `entries`: Individual sample sizes (when `sample_size` is 0).
    #[derive(Debug, Clone)]
    pub struct StszBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: StszFlags,
        /// Default sample size if all samples are equal, 0 for variable.
        pub sample_size: u32,
        /// Individual sample sizes (empty when sample_size is non-zero).
        pub entries: Vec<StszEntry>,
    }

    impl From<&StszBoxView<'_>> for StszBox {
        fn from(view: &StszBoxView<'_>) -> Self {
            let entries = view.entries().collect();
            StszBox {
                version: view.version,
                flags: view.flags,
                sample_size: view.sample_size,
                entries,
            }
        }
    }

    impl StszBoxView<'_> {
        /// Converts this view into an owned `StszBox`.
        pub fn to_owned(&self) -> StszBox {
            StszBox::from(self)
        }
    }

    impl BoxCodec for StszBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSZ
        }
    }

    impl BoxDecode<'_> for StszBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StszBoxView::decode(bytes)?;
            Ok(StszBox::from(&view))
        }
    }

    impl BoxEncode for StszBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // sample_size
            + 4 // sample_count
            + (self.entries.len() * 4) // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?; // version
            cur.write_array(&self.flags.to_be_bytes())?; // flags

            cur.write_u32_be(self.sample_size)?;
            cur.write_u32_be(self.entries.len() as u32)?;

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

    fn raw_data() -> [u8; 20] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // sample_size = 0 (variable)
            0x00, 0x00, 0x00, 0x02, // sample_count = 2
            // entries
            0x00, 0x00, 0x01, 0x00, // entry_size = 256
            0x00, 0x00, 0x02, 0x00, // entry_size = 512
        ]
    }

    #[test]
    fn test_stsz_box_view_decode() {
        let data = raw_data();
        let stsz = StszBoxView::decode(&data).unwrap();

        assert_eq!(stsz.version, 0);
        assert_eq!(stsz.sample_size, 0);
        assert_eq!(stsz.sample_count, 2);

        let mut entries = stsz.entries();
        assert_eq!(entries.next().unwrap().entry_size, 256);
        assert_eq!(entries.next().unwrap().entry_size, 512);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stsz_box_view_empty_entries() {
        let data: [u8; 12] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // sample_size = 0
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ];

        let stsz = StszBoxView::decode(&data).unwrap();
        assert_eq!(stsz.sample_count, 0);
        assert_eq!(stsz.entries().count(), 0);
    }

    #[test]
    fn test_stsz_box_view_truncated() {
        let data: [u8; 8] = [0x00; 8];
        let result = StszBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stsz_entry_round_trip() {
        let entry = StszEntry { entry_size: 1024 };

        let bytes = entry.to_bytes();

        let decoded = StszEntry::from_bytes(&bytes);
        assert_eq!(decoded.entry_size, entry.entry_size);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsz_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stsz = StszBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stsz.encoded_len()];
        stsz.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsz_box_to_owned() {
        let data = raw_data();
        let view = StszBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.sample_size, view.sample_size);
        assert_eq!(owned.entries.len(), view.sample_count as usize);
    }
}
