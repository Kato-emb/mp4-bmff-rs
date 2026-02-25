//! Degradation Priority Box (`stdp`) implementation.
//!
//! The Degradation Priority Box assigns a relative priority to each sample,
//! indicating which samples are more important to preserve when the media must
//! be degraded (e.g., under bandwidth constraints or for transcoding). Higher
//! priority values indicate more important samples that should be retained.
//!
//! This box is optional and resides within the Sample Table Box (`stbl`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedEntry;

define_box_flags!(
    /// Flags for the Degradation Priority Box (`stdp`).
    ///
    /// Reserved (should be 0).
    StdpFlags {}
);

/// An entry in the Degradation Priority Box (`stdp`).
///
/// Contains a 16-bit degradation priority value for a single sample.
/// Higher values indicate greater importance - samples with higher
/// priorities should be retained preferentially when degradation occurs.
#[derive(Debug, Clone, Copy)]
pub struct StdpEntry {
    /// The degradation priority of the sample (higher = more important).
    pub priority: u16,
}

impl FixedEntry<2> for StdpEntry {
    fn from_bytes(bytes: &[u8; 2]) -> Self {
        StdpEntry {
            priority: u16::from_be_bytes([bytes[0], bytes[1]]),
        }
    }

    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> [u8; 2] {
        self.priority.to_be_bytes()
    }
}

define_entry_iter!(
    /// An iterator over entries in the Degradation Priority Box (`stdp`).
    pub struct StdpEntryIter(StdpEntry, 2);
);

/// A reference to a Degradation Priority Box (`stdp`).
///
/// Assigns degradation priorities to samples, allowing decoders or
/// transcoders to make informed decisions about which samples to preserve
/// when quality must be reduced.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `entries`: One 16-bit priority per sample (indexed by sample number).
#[derive(Debug)]
pub struct StdpBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: StdpFlags,
    entries: &'a [u8],
}

impl<'a> StdpBoxView<'a> {
    /// Returns an iterator over the STDP entries.
    pub fn entries(&self) -> StdpEntryIter<'a> {
        StdpEntryIter::new(self.entries)
    }
}

impl BoxCodec for StdpBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STDP
    }
}

impl<'de> BoxDecode<'de> for StdpBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = crate::cursor::ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;

        // Read flags (3 bytes)
        let flags = StdpFlags::from_be_bytes(cur.read_array::<3>()?);

        if !cur.remaining().is_multiple_of(StdpEntry::ENTRY_SIZE) {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "STDP entries length is not a multiple of entry size",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STDP,
            ));
        }

        // Read entries
        let entries = cur.take(cur.remaining())?;

        Ok(StdpBoxView {
            version,
            flags,
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

    /// An owned Degradation Priority Box (`stdp`).
    ///
    /// This is the owned variant of [`StdpBoxView`] that stores degradation
    /// priority entries in a heap-allocated vector.
    ///
    /// # Structure
    ///
    /// - `version`: Box version (should be 0).
    /// - `flags`: Reserved (should be 0).
    /// - `entries`: Degradation priority for each sample.
    #[derive(Debug, Clone)]
    pub struct StdpBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: StdpFlags,
        /// Degradation priority entries (one per sample).
        pub entries: Vec<StdpEntry>,
    }

    impl From<&StdpBoxView<'_>> for StdpBox {
        fn from(view: &StdpBoxView<'_>) -> Self {
            let entries = view.entries().collect();

            StdpBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl StdpBoxView<'_> {
        /// Converts this view into an owned `StdpBox`.
        pub fn to_owned(&self) -> StdpBox {
            StdpBox::from(self)
        }
    }

    impl BoxCodec for StdpBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STDP
        }
    }

    impl BoxDecode<'_> for StdpBox {
        fn decode(bytes: &'_ [u8]) -> Result<Self> {
            let view = StdpBoxView::decode(bytes)?;
            Ok(StdpBox::from(&view))
        }
    }

    impl BoxEncode for StdpBox {
        fn encoded_len(&self) -> usize {
            4 + self.entries.len() * StdpEntry::ENTRY_SIZE
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

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

    fn raw_data() -> [u8; 8] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x64, // entry 1: priority = 100
            0x00, 0xC8, // entry 2: priority = 200
        ]
    }

    #[test]
    fn test_stdp_box_view_decode() {
        let data = raw_data();
        let stdp = StdpBoxView::decode(&data).unwrap();

        assert_eq!(stdp.version, 0);
        assert_eq!(stdp.flags.bits(), 0);

        let mut entries = stdp.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.priority, 100);
        let second = entries.next().unwrap();
        assert_eq!(second.priority, 200);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stdp_box_view_empty_entries() {
        let data: [u8; 4] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
        ];

        let stdp = StdpBoxView::decode(&data).unwrap();
        assert_eq!(stdp.version, 0);
        assert_eq!(stdp.entries().count(), 0);
    }

    #[test]
    fn test_stdp_box_view_invalid_size() {
        // 3 bytes after header: not a multiple of 2 (entry size)
        let data: [u8; 7] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x64, // entry 1: priority = 100
            0x00, // incomplete
        ];

        let result = StdpBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stdp_box_view_truncated() {
        let data: [u8; 2] = [0x00, 0x00]; // too short

        let result = StdpBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stdp_entry_round_trip() {
        let entry = StdpEntry { priority: 12345 };

        let bytes = entry.to_bytes();

        let decoded = StdpEntry::from_bytes(&bytes);
        assert_eq!(decoded.priority, entry.priority);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stdp_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stdp = StdpBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stdp.encoded_len()];
        stdp.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stdp_box_to_owned() {
        let data = raw_data();
        let view = StdpBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entries().count());
    }
}
